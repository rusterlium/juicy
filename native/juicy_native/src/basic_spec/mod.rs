use iterative_json_parser::{Parser, Pos, ParseError, Unexpected};

use rustler::{NifEnv, NifTerm, NifResult, NifEncoder};
use rustler::resource::ResourceArc;
use rustler::types::binary::NifBinary;

use ::strings::BuildString;

use ::tree_spec::spec_from_term;
use ::tree_spec::SpecWalker;

use ::input_provider::single::SingleBinaryProvider;

use ::path_tracker::PathTracker;

use std::sync::Mutex;
use std::ops::DerefMut;

mod source_sink;
use self::source_sink::{StreamingSS, SSState};

fn format_unexpected<'a>(env: NifEnv<'a>, pos: Pos, reason: Unexpected) -> NifTerm<'a> {
    let position = pos.0 as u64;
    let explaination = reason.explain().encode(env);
    (::atoms::error(), (::atoms::unexpected(), position, explaination)).encode(env)
}

pub struct BasicSpecIterState {
    parser: Parser,
    ss_state: SSState,
}
pub struct BasicSpecIterStateWrapper(Mutex<BasicSpecIterState>);

pub fn parse_init<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    let binary: NifBinary = args[0].decode()?;
    let spec = spec_from_term(args[1])?;

    let ss_state = SSState {
        path_tracker: PathTracker {
            path: Vec::new(),
            walker: SpecWalker::new(spec),
        },

        position: 0,
        first_needed: 0,
        current_string: BuildString::None,
    };

    let iter_state = BasicSpecIterState {
        parser: Parser::new(),
        ss_state: ss_state,
    };

    let resource = ResourceArc::new(BasicSpecIterStateWrapper(Mutex::new(iter_state)));
    let stack: [u8; 0] = [];
    let state = (binary, &stack as &[u8], resource).encode(env);
    Ok((::atoms::ok(), state).encode(env))
}

pub fn parse_iter<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    let (binary, stack, resource): (NifBinary,
                                    Vec<NifTerm<'a>>,
                                    ResourceArc<BasicSpecIterStateWrapper>) = args[0].decode()?;

    let (res, mut out_stack) = {
        let mut resource_inner_guard = resource.0.lock().unwrap();
        let mut iter_state = resource_inner_guard.deref_mut();

        let mut ss = StreamingSS {
            env: env,
            input: SingleBinaryProvider::new(binary),
            next_reschedule: iter_state.ss_state.position + 40_000,
            out_stack: stack,
            state: &mut iter_state.ss_state,
        };

        let res = iter_state.parser.run(&mut ss);
        (res, ss.out_stack)
    };

    match res {
        Ok(()) => {
            let result = out_stack.pop().unwrap();
            Ok((::atoms::ok(), result).encode(env))
        }
        Err(ParseError::SourceBail(())) => {
            let state = (binary, out_stack, resource).encode(env);
            Ok((::atoms::iter(), state).encode(env))
        }
        Err(ParseError::Unexpected(pos, reason)) => {
            let error = format_unexpected(env, pos, reason);
            Ok((::atoms::error(), error).encode(env))
        }
        Err(_) => panic!("TODO: Add proper error"),
    }
}
