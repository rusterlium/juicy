use std::io::Write;

use super::{InputBinaries, BailType};

use ::strings::BuildString;
use ::numbers::number_data_to_term;

use ::tree_spec::{SpecWalker, ValueType, PathEntry};
use ::tree_spec::NodeId;

use rustler::{NifEnv, NifTerm, NifEncoder};
use rustler::types::map::map_new;
use rustler::types::binary::OwnedNifBinary;

use ::iterative_json_parser::{Bailable, Source, Sink, Pos, PeekResult, Position, NumberData,
                              StringPosition};
use iterative_json_parser::Range as PRange;

pub struct StreamingSS<'a, 'b>
    where 'a: 'b
{
    pub env: NifEnv<'a>,
    pub input: InputBinaries<'a, 'b>,
    pub next_reschedule: usize,
    pub out_stack: Vec<NifTerm<'a>>,
    pub state: &'b mut SSState,
    pub yields: Vec<NifTerm<'a>>,
}

pub struct SSState {
    pub walker: SpecWalker,
    pub path: Vec<PathEntry>,

    pub position: usize,
    pub first_needed: usize,
    pub current_string: BuildString,
}

impl<'a, 'b> Bailable for StreamingSS<'a, 'b> {
    type Bail = BailType;
}

impl<'a, 'b> Source for StreamingSS<'a, 'b> {
    fn position(&self) -> Pos {
        self.state.position.into()
    }
    fn skip(&mut self, num: usize) {
        self.state.position += num
    }
    fn peek_char(&mut self) -> PeekResult<BailType> {
        if self.state.position == self.next_reschedule {
            PeekResult::Bail(BailType::Reschedule)
        } else if let Some(byte) = self.input.byte(self.state.position) {
            PeekResult::Ok(byte)
        } else {
            PeekResult::Bail(BailType::AwaitInput)
        }
    }
    fn peek_slice<'c>(&'c self, _length: usize) -> Option<&'c [u8]> {
        None
    }
}

impl<'a, 'b> StreamingSS<'a, 'b> {
    fn visit_terminal(&mut self, _pos: Position, typ: ValueType) -> Result<(), BailType> {
        let node_id = self.state.walker.visit_terminal(typ, self.state.path.last());
        let res = self.do_stream(node_id);
        self.update_path();
        res
    }

    fn enter_nonterminal(&mut self, _pos: Position, typ: ValueType) {
        let last_key = self.state.path.last();
        self.state.walker.enter_nonterminal(typ, last_key);
    }

    fn exit_nonterminal(&mut self) -> Result<(), BailType> {
        let node_id = self.state.walker.exit_nonterminal();
        let res = self.do_stream(node_id);
        self.update_path();
        res
    }

    fn update_path(&mut self) {
        match self.state.path.pop() {
            Some(PathEntry::Index(index)) => self.state.path.push(PathEntry::Index(index + 1)),
            Some(_) => (),
            None => (),
        }
    }

    fn do_stream(&mut self, node_id_opt: Option<NodeId>) -> Result<(), BailType> {
        match node_id_opt {
            Some(node_id) => {
                let node = self.state.walker.spec.get(node_id);
                if node.options.stream {
                    let path = self.state.path.encode(self.env);
                    let term = self.out_stack.pop().unwrap();
                    self.out_stack.push(::atoms::streamed().encode(self.env));
                    self.yields.push((::atoms::yield_(), (path, term)).encode(self.env))
                }
            }
            None => (),
        }
        Ok(())
    }
}

impl<'a, 'b> Sink for StreamingSS<'a, 'b> {
    fn push_map(&mut self, pos: Position) {
        self.out_stack.push(map_new(self.env));
        self.state.first_needed = self.state.position;

        self.enter_nonterminal(pos, ValueType::Object);
    }
    fn push_array(&mut self, pos: Position) {
        let arr: Vec<NifTerm> = Vec::new();
        self.out_stack.push(arr.encode(self.env));
        self.enter_nonterminal(pos, ValueType::Array);
        self.state.first_needed = self.state.position;

        self.state.path.push(PathEntry::Index(0));
    }
    fn push_number(&mut self, pos: Position, num: NumberData) -> Result<(), Self::Bail> {
        let term = number_data_to_term(self.env, num, |r, b| self.input.push_range(r, b));
        self.out_stack.push(term);
        self.state.first_needed = self.state.position;

        self.visit_terminal(pos, ValueType::Number)?;

        Ok(())
    }
    fn push_bool(&mut self, pos: Position, val: bool) -> Result<(), Self::Bail> {
        self.out_stack.push(val.encode(self.env));

        self.state.first_needed = self.state.position;
        self.visit_terminal(pos, ValueType::Boolean)?;

        Ok(())
    }
    fn push_null(&mut self, pos: Position) -> Result<(), Self::Bail> {
        self.out_stack.push(::atoms::nil().encode(self.env));

        self.state.first_needed = self.state.position;
        self.visit_terminal(pos, ValueType::Null)?;

        Ok(())
    }

    fn start_string(&mut self, pos: StringPosition) {
        self.state.current_string = match pos {
            StringPosition::MapKey => BuildString::new_owned(),
            _ => BuildString::new(),
        };
    }
    fn append_string_range(&mut self, range: PRange) {
        let input = &self.input;
        self.state.current_string.append_range(range, |r, b| input.push_range(r, b));
    }
    fn append_string_single(&mut self, character: u8) {
        let input = &self.input;
        self.state.current_string.append_single(character, |r, b| input.push_range(r, b));
    }
    fn append_string_codepoint(&mut self, codepoint: char) {
        let input = &self.input;
        self.state.current_string.append_codepoint(codepoint, |r, b| input.push_range(r, b));
    }
    fn finalize_string(&mut self, pos: StringPosition) -> Result<(), Self::Bail> {
        match pos {
            StringPosition::MapKey => {
                let string = ::std::mem::replace(&mut self.state.current_string, BuildString::None);
                let key = string.owned_to_vec();

                let mut bin = OwnedNifBinary::new(key.len()).unwrap();
                bin.as_mut_slice().write(&key).unwrap();
                self.out_stack.push(bin.release(self.env).encode(self.env));

                self.state.path.push(PathEntry::Key(key));
            }
            _ => {
                let string_term = match self.state.current_string {
                    BuildString::None => "".encode(self.env),
                    BuildString::Range(range) => self.input.range_to_term(self.env, range),
                    BuildString::Owned(ref buf) => {
                        let mut bin = OwnedNifBinary::new(buf.len()).unwrap();
                        bin.as_mut_slice().write(buf).unwrap();
                        bin.release(self.env).encode(self.env)
                    }
                };
                self.state.current_string = BuildString::None;
                self.out_stack.push(string_term);

                self.visit_terminal(pos.to_position(), ValueType::String)?;
            }
        }
        self.state.first_needed = self.state.position;
        Ok(())
    }

    fn finalize_map(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        self.exit_nonterminal()?;

        self.state.first_needed = self.state.position;

        Ok(())
    }
    fn finalize_array(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        let term = self.out_stack.pop().unwrap();
        self.out_stack.push(term.list_reverse().ok().unwrap());

        self.state.path.pop().unwrap();

        self.state.first_needed = self.state.position;

        self.exit_nonterminal()?;

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
