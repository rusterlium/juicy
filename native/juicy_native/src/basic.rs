use iterative_json_parser::{Parser, Source, PeekResult, Sink, Range, Pos, NumberData, ParseError,
                            Unexpected, Bailable, Position, StringPosition};

use rustler::{NifEnv, NifTerm, NifResult, NifEncoder};
use rustler::resource::ResourceArc;
use rustler::types::binary::NifBinary;
use rustler::types::map::map_new;
use rustler::types::binary::OwnedNifBinary;

use ::strings::BuildString;
use ::numbers::number_data_to_term;
use ::input_provider::InputProvider;
use ::input_provider::single::SingleBinaryProvider;

use std::io::Write;
use std::sync::Mutex;
use std::ops::DerefMut;

struct BasicSS<'a, 'b> {
    env: NifEnv<'a>,

    input: SingleBinaryProvider<'a>,

    position: usize,
    next_reschedule: usize,

    out_stack: Vec<NifTerm<'a>>,
    current_string: &'b mut BuildString,
}

impl<'a, 'b> Bailable for BasicSS<'a, 'b> {
    type Bail = ();
}
impl<'a, 'b> Source for BasicSS<'a, 'b> {
    fn position(&self) -> Pos {
        self.position.into()
    }
    fn skip(&mut self, num: usize) {
        self.position += num
    }
    fn peek_char(&mut self) -> PeekResult<()> {
        if self.position == self.next_reschedule {
            PeekResult::Bail(())
        } else if let Some(character) = self.input.byte(self.position) {
            PeekResult::Ok(character)
        } else {
            PeekResult::Eof
        }
    }
    fn peek_slice<'c>(&'c self, _length: usize) -> Option<&'c [u8]> {
        // let (_, slice) = self.source.split_at(self.position);
        // if slice.len() >= length {
        //    Some(slice)
        // else {
        //    None
        //
        None
    }
}

impl<'a, 'b> Sink for BasicSS<'a, 'b> {
    fn push_map(&mut self, _pos: Position) {
        self.out_stack.push(map_new(self.env));
    }
    fn push_array(&mut self, _pos: Position) {
        let arr: Vec<NifTerm> = Vec::new();
        self.out_stack.push(arr.encode(self.env));
    }
    fn push_number(&mut self, _pos: Position, num: NumberData) -> Result<(), Self::Bail> {
        let term = number_data_to_term(self.env, num, |r, b| {
            self.input.push_range(r, b);
        });
        self.out_stack.push(term);
        Ok(())
    }
    fn push_bool(&mut self, _pos: Position, val: bool) -> Result<(), Self::Bail> {
        self.out_stack.push(val.encode(self.env));
        Ok(())
    }
    fn push_null(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        self.out_stack.push(::atoms::nil().encode(self.env));
        Ok(())
    }

    fn start_string(&mut self, _pos: StringPosition) {
        *self.current_string = BuildString::new();
    }
    fn append_string_range(&mut self, range: Range) {
        let input = &self.input;
        self.current_string.append_range(range, |r, b| {
            input.push_range(r, b);
        });
    }
    fn append_string_single(&mut self, character: u8) {
        let input = &self.input;
        self.current_string.append_single(character, |r, b| {
            input.push_range(r, b);
        });
    }
    fn append_string_codepoint(&mut self, codepoint: char) {
        let input = &self.input;
        self.current_string.append_codepoint(codepoint, |r, b| {
            input.push_range(r, b);
        });
    }
    fn finalize_string(&mut self, _pos: StringPosition) -> Result<(), Self::Bail> {
        let string_term = match *self.current_string {
            BuildString::None => "".encode(self.env),
            BuildString::Range(range) => self.input.range_to_term(self.env, range),
            BuildString::Owned(ref buf) => {
                let mut bin = OwnedNifBinary::new(buf.len()).unwrap();
                bin.as_mut_slice().write(buf).unwrap();
                bin.release(self.env).encode(self.env)
            }
        };
        *self.current_string = BuildString::None;
        self.out_stack.push(string_term);
        Ok(())
    }

    fn finalize_map(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        Ok(())
    }
    fn finalize_array(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        let term = self.out_stack.pop().unwrap();
        self.out_stack.push(term.list_reverse().ok().unwrap());
        Ok(())
    }
    fn pop_into_map(&mut self) {
        let value = self.out_stack.pop().unwrap();
        let key = self.out_stack.pop().unwrap();
        let map = self.out_stack.pop().unwrap();
        self.out_stack.push(map.map_put(key, value).ok().unwrap());
    }
    fn pop_into_array(&mut self) {
        let value = self.out_stack.pop().unwrap();
        let array = self.out_stack.pop().unwrap();
        self.out_stack.push(array.list_prepend(value));
    }
}

fn format_unexpected<'a>(env: NifEnv<'a>,
                         parser: &Parser,
                         pos: Pos,
                         reason: Unexpected)
                         -> NifTerm<'a> {
    let parser_state = format!("{:?}", parser).encode(env);
    let position = pos.0 as u64;
    let explaination = reason.explain().encode(env);
    (::atoms::error(), (::atoms::unexpected(), position, explaination, parser_state)).encode(env)
}

pub struct IterState {
    parser: Parser,
    source_pos: usize,
    sink_string_state: BuildString,
}
pub struct IterStateWrapper(Mutex<IterState>);

fn parse_inner<'a>(env: NifEnv<'a>,
                   input: NifBinary<'a>,
                   stack: Vec<NifTerm<'a>>,
                   iter_state: &mut IterState)
                   -> Result<NifTerm<'a>, Vec<NifTerm<'a>>> {
    let mut ss = BasicSS {
        env: env,
        input: SingleBinaryProvider::new(input),
        position: iter_state.source_pos,
        next_reschedule: iter_state.source_pos + 40_000,
        out_stack: stack,
        current_string: &mut iter_state.sink_string_state,
    };

    let result = iter_state.parser.run(&mut ss);
    iter_state.source_pos = ss.position;

    match result {
        Ok(()) => {
            let term = ss.out_stack.pop().unwrap();
            Ok((::atoms::ok(), term).encode(env))
        }
        Err(ParseError::SourceBail(())) => {
            Err(ss.out_stack)
        }
        Err(ParseError::Unexpected(pos, reason)) => {
            Ok(format_unexpected(env, &iter_state.parser, pos, reason))
        }
        err => panic!("{:?}", err),
    }
}

pub fn parse<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    let input: NifBinary = args[0].decode()?;

    let mut iter_state = IterState {
        parser: Parser::new(),
        source_pos: 0,
        sink_string_state: BuildString::None,
    };

    match parse_inner(env, input, vec![], &mut iter_state) {
        Ok(res) => Ok(res),
        Err(stack) => {
            let resource = ResourceArc::new(IterStateWrapper(Mutex::new(iter_state)));
            Ok((::atoms::iter(), stack, resource).encode(env))
        }
    }
}

pub fn parse_iter<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    let input: NifBinary = args[0].decode()?;
    let stack: Vec<NifTerm<'a>> = args[1].decode()?;
    let resource: ResourceArc<IterStateWrapper> = args[2].decode()?;
    let mut resource_inner_guard = resource.0.lock().unwrap();
    let mut resource_inner = resource_inner_guard.deref_mut();

    match parse_inner(env, input, stack, resource_inner) {
        Ok(res) => Ok(res),
        Err(stack) => Ok((::atoms::iter(), stack, args[2]).encode(env)),
    }
}
