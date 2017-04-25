use ::iterative_json_parser::Range;
use ::std::io::Write;

use ::rustler::{NifTerm, NifEnv, NifEncoder};
use ::rustler::types::binary::OwnedNifBinary;

use ::input_provider::InputProvider;

pub enum BuildString {
    None,
    Range(Range),
    Owned(Vec<u8>),
}

impl BuildString {

    pub fn new() -> BuildString {
        BuildString::None
    }

    pub fn new_owned() -> BuildString {
        BuildString::Owned(Vec::new())
    }

    pub fn append_range<'a, F>(&'a mut self, range: Range, range_provider: F)
        where F: Fn(Range, &mut Vec<u8>) {

        match *self {
            BuildString::None => {
                *self = BuildString::Range(range);
            },
            BuildString::Range(prev_range) => {
                let mut buf: Vec<u8> = Vec::new();
                range_provider(prev_range, &mut buf);
                range_provider(range, &mut buf);
                *self = BuildString::Owned(buf);
            },
            BuildString::Owned(ref mut buf) => {
                range_provider(range, buf);
            },
        }
    }

    pub fn append_single<'a, F>(&'a mut self, single: u8, range_provider: F)
        where F: Fn(Range, &mut Vec<u8>) {

        match *self {
            BuildString::None => {
                *self = BuildString::Owned(vec![single]);
            },
            BuildString::Range(prev_range) => {
                let mut buf: Vec<u8> = Vec::new();
                range_provider(prev_range, &mut buf);
                buf.push(single);
                *self = BuildString::Owned(buf);
            },
            BuildString::Owned(ref mut buf) => {
                buf.push(single);
            },
        }
    }

    pub fn append_codepoint<'a, F>(&'a mut self, codepoint: char, range_provider: F)
        where F: Fn(Range, &mut Vec<u8>) {

        let mut buf: [u8; 4] = [0, 0, 0, 0];
        let codepoint_slice = codepoint.encode_utf8(&mut buf);

        match *self {
            BuildString::None => {
                let mut vec = Vec::<u8>::new();
                vec.extend_from_slice(codepoint_slice.as_bytes());
                *self = BuildString::Owned(vec);
            },
            BuildString::Range(prev_range) => {
                let mut buf: Vec<u8> = Vec::new();
                range_provider(prev_range, &mut buf);
                buf.extend_from_slice(codepoint_slice.as_bytes());
                *self = BuildString::Owned(buf);
            },
            BuildString::Owned(ref mut buf) => {
                buf.extend_from_slice(codepoint_slice.as_bytes());
            },
        }
    }

    pub fn owned_to_vec(self) -> Vec<u8> {
        match self {
            BuildString::Owned(vec) => vec,
            _ => panic!(),
        }
    }

    pub fn to_term<'a, T, M>(self, input: &mut T, env: NifEnv<'a>) -> NifTerm<'a> where T: InputProvider<M> {
        match self {
            BuildString::None => "".encode(env),
            BuildString::Range(range) => input.range_to_term(env, range),
            BuildString::Owned(ref buf) => {
                let mut bin = OwnedNifBinary::new(buf.len()).unwrap();
                bin.as_mut_slice().write(buf).unwrap();
                bin.release(env).encode(env)
            }
        }
    }

}
