use std::io::Write;

use ::strings::BuildString;
use ::numbers::number_data_to_term;

use ::tree_spec::ValueType;

use rustler::{NifEnv, NifTerm, NifEncoder};
use rustler::types::map::map_new;
use rustler::types::binary::OwnedNifBinary;

use ::iterative_json_parser::{Bailable, Source, Sink, Pos, PeekResult, Position, NumberData,
                              StringPosition};
use iterative_json_parser::Range as PRange;

use ::input_provider::InputProvider;
use ::input_provider::single::SingleBinaryProvider;

use ::path_tracker::PathTracker;

pub struct StreamingSS<'a, 'b>
    where 'a: 'b
{
    pub env: NifEnv<'a>,
    pub input: SingleBinaryProvider<'a>,
    pub next_reschedule: usize,
    pub out_stack: Vec<NifTerm<'a>>,
    pub state: &'b mut SSState,
}

pub struct SSState {
    pub path_tracker: PathTracker,

    pub position: usize,
    pub first_needed: usize,
    pub current_string: BuildString,
}

impl<'a, 'b> Bailable for StreamingSS<'a, 'b> {
    type Bail = ();
}

impl<'a, 'b> Source for StreamingSS<'a, 'b> {
    fn position(&self) -> Pos {
        self.state.position.into()
    }
    fn skip(&mut self, num: usize) {
        self.state.position += num
    }
    fn peek_char(&mut self) -> PeekResult<()> {
        if self.state.position == self.next_reschedule {
            PeekResult::Bail(())
        } else {
            match self.input.byte(self.state.position) {
                Some(byte) => PeekResult::Ok(byte),
                None => unreachable!(),
            }
        }
    }
    fn peek_slice<'c>(&'c self, _length: usize) -> Option<&'c [u8]> {
        None
    }
}

impl<'a, 'b> Sink for StreamingSS<'a, 'b> {
    fn push_map(&mut self, pos: Position) {
        self.out_stack.push(map_new(self.env));

        self.state.path_tracker.enter_map(pos);
        self.state.first_needed = self.state.position;
    }
    fn push_array(&mut self, pos: Position) {
        let arr: Vec<NifTerm> = Vec::new();
        self.out_stack.push(arr.encode(self.env));

        self.state.path_tracker.enter_array(pos);
        self.state.first_needed = self.state.position;
    }
    fn push_number(&mut self, pos: Position, num: NumberData) -> Result<(), Self::Bail> {
        let term = number_data_to_term(self.env, num, |r, b| self.input.push_range(r, b));
        self.out_stack.push(term);

        let curr_node = self.state.path_tracker.visit_terminal(pos, ValueType::Number);
        //self.do_stream(curr_node)?;

        self.state.first_needed = self.state.position;
        Ok(())
    }
    fn push_bool(&mut self, pos: Position, val: bool) -> Result<(), Self::Bail> {
        self.out_stack.push(val.encode(self.env));

        let curr_node = self.state.path_tracker.visit_terminal(pos, ValueType::Boolean);
        //self.do_stream(curr_node)?;

        self.state.first_needed = self.state.position;
        Ok(())
    }
    fn push_null(&mut self, pos: Position) -> Result<(), Self::Bail> {
        self.out_stack.push(::atoms::nil().encode(self.env));

        let curr_node = self.state.path_tracker.visit_terminal(pos, ValueType::Null);
        //self.do_stream(curr_node)?;

        self.state.first_needed = self.state.position;
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
        let string = ::std::mem::replace(&mut self.state.current_string, BuildString::None);

        match pos {
            StringPosition::MapKey => {
                let key = string.owned_to_vec();

                let curr_node_id = self.state.path_tracker.enter_key(key.clone());
                let key_atom = curr_node_id
                    .and_then(|node_id| {
                        let curr_node = self.state.path_tracker.walker.spec.get(node_id);
                        match curr_node.options.atom_mappings {
                            Some(ref some) => some.get(&key).cloned(),
                            None => None,
                        }
                    });

                if let Some(atom) = key_atom {
                    self.out_stack.push(atom.encode(self.env));
                } else {
                    let mut bin = OwnedNifBinary::new(key.len()).unwrap();
                    bin.as_mut_slice().write(&key).unwrap();
                    self.out_stack.push(bin.release(self.env).encode(self.env));
                }
            }
            _ => {
                let string_term = string.to_term(&mut self.input, self.env);
                self.out_stack.push(string_term);

                let curr_node = self.state.path_tracker.visit_terminal(pos.to_position(), ValueType::String);
            }
        }
        self.state.first_needed = self.state.position;
        Ok(())
    }

    fn finalize_map(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        self.state.first_needed = self.state.position;

        let curr_node_id = self.state.path_tracker.exit_map();

        let struct_atom = curr_node_id.current
            .and_then(|node_id| {
                let curr_node = self.state.path_tracker.walker.spec.get(node_id);
                match curr_node.options.struct_atom {
                    Some(ref atom) => Some(atom.clone()),
                    None => None,
                }
            });

        if let Some(atom) = struct_atom {
            let term = self.out_stack.pop().unwrap();
            self.out_stack.push(term.map_put(
                ::atoms::__struct__().encode(self.env),
                atom.encode(self.env)).ok().unwrap());
        }

        Ok(())
    }
    fn finalize_array(&mut self, _pos: Position) -> Result<(), Self::Bail> {
        let term = self.out_stack.pop().unwrap();
        self.out_stack.push(term.list_reverse().ok().unwrap());

        self.state.first_needed = self.state.position;

        let curr_node = self.state.path_tracker.exit_array();

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
