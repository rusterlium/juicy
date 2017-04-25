use ::iterative_json_parser::Range as PRange;
use ::rustler::{NifTerm, NifEnv};

pub mod single;
pub mod streaming;

/// Things implementing this trait are responsible for providing
/// input data to both the parser and to the code constructing terms
/// from the parser output.
///
/// It is not intended to be fully generic and swappable (hence why it
/// is generic over `DataResponse`), but rather as a way to encapsulate
/// this logic. It is intended to be an easily usable building block
/// when writing `SourceSink`s.
pub trait InputProvider<DataResponse> {
    fn byte(&self, pos: usize) -> DataResponse;
    fn push_range(&self, range: PRange, buf: &mut Vec<u8>);
    fn range_to_term<'a>(&self, env: NifEnv<'a>, range: PRange) -> NifTerm<'a>;
}
