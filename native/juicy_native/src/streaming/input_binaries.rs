use ::std::ops::Range;
use std::io::Write;
use ::rustler::{NifEnv, NifTerm, NifEncoder};
use ::rustler::types::binary::{NifBinary, OwnedNifBinary};
use ::iterative_json_parser::Range as PRange;

pub struct InputBinaries<'a, 'b>
    where 'a: 'b
{
    pub binaries: &'b [(Range<usize>, NifBinary<'a>)],
}
impl<'a, 'b> InputBinaries<'a, 'b> {
    pub fn byte(&self, pos: usize) -> Option<u8> {
        for &(ref range, bin) in self.binaries {
            if range.start <= pos && range.end > pos {
                return Some(bin.as_slice()[pos - range.start]);
            }
        }
        None
    }

    pub fn push_range(&self, range: PRange, buf: &mut Vec<u8>) {
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

    pub fn range_to_term<'c>(&self, env: NifEnv<'c>, range: PRange) -> NifTerm<'c> {
        // TODO
        let mut buf: Vec<u8> = Vec::new();
        self.push_range(range, &mut buf);

        let mut bin = OwnedNifBinary::new(buf.len()).unwrap();
        bin.as_mut_slice().write(&buf).unwrap();
        bin.release(env).encode(env)
    }
}
