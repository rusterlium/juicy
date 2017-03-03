use iterative_json_parser::{Parser, Pos, ParseError, Unexpected};

use rustler::{NifEnv, NifTerm, NifResult, NifEncoder};
use rustler::resource::ResourceArc;
use rustler::types::binary::NifBinary;
use rustler::types::binary::OwnedNifBinary;
use rustler::types::list::NifListIterator;

use ::strings::BuildString;

use ::tree_spec::spec_from_term;
use ::tree_spec::{SpecWalker, PathEntry};

use std::io::Write;
use std::sync::Mutex;
use std::ops::DerefMut;
use std::ops::Range;

mod input_binaries;
use self::input_binaries::InputBinaries;
mod source_sink;
use self::source_sink::{StreamingSS, SSState};

impl NifEncoder for PathEntry {
    fn encode<'a>(&self, env: NifEnv<'a>) -> NifTerm<'a> {
        match self {
            &PathEntry::Index(idx) => (idx as u64).encode(env),
            &PathEntry::Key(ref key) => {
                let mut bin = OwnedNifBinary::new(key.len()).unwrap();
                bin.as_mut_slice().write(key).unwrap();
                bin.release(env).encode(env)
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum BailType {
    Reschedule,
    AwaitInput,
}

fn format_unexpected<'a>(env: NifEnv<'a>, pos: Pos, reason: Unexpected) -> NifTerm<'a> {
    let position = pos.0 as u64;
    let explaination = reason.explain().encode(env);
    (::atoms::error(), (::atoms::unexpected(), position, explaination)).encode(env)
}


pub struct StreamingIterState {
    parser: Parser,
    ss_state: SSState,
}
pub struct StreamingIterStateWrapper(Mutex<StreamingIterState>);


fn read_binaries<'a>(term: NifTerm<'a>) -> NifResult<Vec<(Range<usize>, NifBinary<'a>)>> {
    let binaries_iter: NifListIterator = term.decode()?;
    let mut binaries_ranges: Vec<(Range<usize>, NifBinary)> = Vec::new();
    for term in binaries_iter {
        let (start, bin): (usize, NifBinary) = term.decode()?;
        let range = start..(start + bin.len());
        binaries_ranges.push((range, bin));
    }
    Ok(binaries_ranges)
}

fn write_binaries<'a>(env: NifEnv<'a>,
                      binaries: &Vec<(Range<usize>, NifBinary<'a>)>,
                      last_needed: usize)
                      -> NifTerm<'a> {
    let res: Vec<NifTerm> = binaries.iter()
        .filter(|&&(ref range, _)| range.end >= last_needed)
        .map(|&(ref range, bin)| (range.start, bin).encode(env))
        .collect();
    res.encode(env)
}

pub fn parse_init<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    let spec = spec_from_term(args[0])?;

    let ss_state = SSState {
        walker: SpecWalker::new(spec),
        path: Vec::new(),

        position: 0,
        first_needed: 0,
        current_string: BuildString::None,
    };

    let iter_state = StreamingIterState {
        parser: Parser::new(),
        ss_state: ss_state,
    };

    let resource = ResourceArc::new(StreamingIterStateWrapper(Mutex::new(iter_state)));
    let stack: [u8; 0] = [];
    let state = (&stack as &[u8], resource).encode(env);
    Ok((::atoms::ok(), state).encode(env))
}

pub fn parse_iter<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    let binaries_ranges: Vec<(Range<usize>, NifBinary)> = read_binaries(args[0])?;
    let (stack, resource): (Vec<NifTerm<'a>>, ResourceArc<StreamingIterStateWrapper>) =
        args[1].decode()?;

    let (res, out_stack, mut yields, first_needed) = {
        let mut resource_inner_guard = resource.0.lock().unwrap();
        let mut iter_state = resource_inner_guard.deref_mut();

        let mut ss = StreamingSS {
            env: env,
            input: InputBinaries { binaries: &binaries_ranges },
            next_reschedule: iter_state.ss_state.position + 40_000,
            out_stack: stack,
            state: &mut iter_state.ss_state,
            yields: Vec::new(),
        };

        let res = iter_state.parser.run(&mut ss);
        (res, ss.out_stack, ss.yields, ss.state.first_needed)
    };

    let binaries_out = write_binaries(env, &binaries_ranges, first_needed);

    match res {
        Ok(()) => {
            yields.push(::atoms::finished().encode(env));
            let state = (out_stack, resource).encode(env);
            Ok((::atoms::finished(), yields, binaries_out, state).encode(env))
        }
        Err(ParseError::SourceBail(BailType::Reschedule)) => {
            let state = (out_stack, resource).encode(env);
            Ok((::atoms::iter(), yields, binaries_out, state).encode(env))
        }
        Err(ParseError::SourceBail(BailType::AwaitInput)) => {
            let state = (out_stack, resource).encode(env);
            Ok((::atoms::await_input(), yields, binaries_out, state).encode(env))
        }
        Err(ParseError::Unexpected(pos, reason)) => {
            let error = format_unexpected(env, pos, reason);
            yields.push(error);
            let state = (out_stack, resource).encode(env);
            Ok((::atoms::finished(), yields, binaries_out, state).encode(env))
        }
        Err(_) => panic!("TODO: Add proper error"),
    }
}
