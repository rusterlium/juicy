#[macro_use]
extern crate rustler;
#[macro_use]
extern crate rustler_codegen;
#[macro_use]
extern crate lazy_static;

extern crate num_traits;
extern crate num_bigint;

use rustler::{NifEnv, NifTerm, NifResult, NifEncoder};

extern crate iterative_json_parser;

mod numbers;
mod strings;
mod basic;
mod streaming;
mod tree_spec;

mod atoms {
    rustler_atoms! {
        atom ok;
        atom nil;
        atom error;
        atom unexpected;
        atom iter;
        atom streamed;
        atom yield_ = "yield";
        atom await_input;
        atom finished;
    }
}

rustler_export_nifs! {
    "Elixir.Juicy.Native",
    [("parse_init", 1, basic::parse),
     ("parse_iter", 3, basic::parse_iter),
     ("stream_parse_init", 1, streaming::parse_init),
     ("stream_parse_iter", 2, streaming::parse_iter),
     ("validate_spec", 1, validate_spec)],
    Some(on_init)
}

fn validate_spec<'a>(env: NifEnv<'a>, args: &[NifTerm<'a>]) -> NifResult<NifTerm<'a>> {
    match tree_spec::spec_from_term(args[0]) {
        Ok(_) => Ok(atoms::ok().encode(env)),
        Err(_) => Ok(atoms::error().encode(env)),
    }
}

fn on_init<'a>(env: NifEnv<'a>, _load_info: NifTerm<'a>) -> bool {
    resource_struct_init!(basic::IterStateWrapper, env);
    resource_struct_init!(streaming::StreamingIterStateWrapper, env);
    true
}
