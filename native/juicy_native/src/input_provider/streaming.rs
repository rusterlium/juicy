use std::ops::Range;
use std::io::Write;

use ::rustler::{NifEnv, NifTerm, NifEncoder};
use ::rustler::types::binary::{NifBinary, OwnedNifBinary};

use super::InputProvider;

use ::iterative_json_parser::Range as PRange;

pub enum StreamingInputResult {
    Ok(u8),
    AwaitInput,
    Eof,
}

/// Provides input from a set of binaries.
pub struct StreamingInputProvider<'a, 'b> where 'a: 'b {
    pub binaries: &'b [(Range<usize>, NifBinary<'a>)]
}

impl<'a, 'b> InputProvider<StreamingInputResult> for StreamingInputProvider<'a, 'b> {

    fn byte(&self, pos: usize) -> StreamingInputResult {
        for &(ref range, bin) in self.binaries {
            if range.start <= pos && range.end > pos {
                return StreamingInputResult::Ok(bin.as_slice()[pos - range.start]);
            }
        }
        StreamingInputResult::AwaitInput
    }

    fn push_range(&self, range: PRange, buf: &mut Vec<u8>) {
        for &(ref b_range, bin) in self.binaries.iter().rev() {

            let s = if range.start < b_range.start {
                0
            } else {
                range.start - b_range.start
            };
            let e = range.end.wrapping_sub(b_range.start);

            let ss = if s < bin.len() { s } else { bin.len() };
            let ee = if e < bin.len() { e } else { bin.len() };

            let slice = &bin.as_slice()[ss..ee];
            buf.extend_from_slice(&slice);
        }
    }

    fn range_to_term<'c>(&self, env: NifEnv<'c>, range: PRange) -> NifTerm<'c> {
        // TODO
        let mut buf: Vec<u8> = Vec::new();
        self.push_range(range, &mut buf);

        let mut bin = OwnedNifBinary::new(buf.len()).unwrap();
        bin.as_mut_slice().write(&buf).unwrap();
        bin.release(env).encode(env)
    }

}
