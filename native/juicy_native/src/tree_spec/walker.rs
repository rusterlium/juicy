use super::{Spec, NodeId, ValueType};

use ::rustler::{NifEnv, NifTerm, NifEncoder};
use ::rustler::types::binary::OwnedNifBinary;
use std::io::Write;

#[derive(Debug)]
pub enum PathEntry {
    Key(Vec<u8>),
    /// The index field is 1 indexed so that 0 can be used as a sentinel value.
    /// This is a massive hack and should probably be fixed. I don't like it.
    Index(usize),
}

impl NifEncoder for PathEntry {
    fn encode<'a>(&self, env: NifEnv<'a>) -> NifTerm<'a> {
        match self {
            &PathEntry::Index(idx) => ((idx - 1) as u64).encode(env),
            &PathEntry::Key(ref key) => {
                let mut bin = OwnedNifBinary::new(key.len()).unwrap();
                bin.as_mut_slice().write(key).unwrap();
                bin.release(env).encode(env)
            }
        }
    }
}

impl PathEntry {
    pub fn key<'a>(&'a self) -> &'a [u8] {
        match self {
            &PathEntry::Key(ref data) => &data,
            _ => unreachable!(),
        }
    }
    pub fn index(&self) -> usize {
        match self {
            &PathEntry::Index(index) => index,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct SpecWalker {
    pub spec: Spec,
    current: NodeId,
    depth: usize,
    height_off_current: usize,
}

pub struct PathPosition {
    pub current: Option<NodeId>,
    pub parent: Option<NodeId>,
}

impl SpecWalker {
    pub fn new(spec: Spec) -> SpecWalker {
        SpecWalker {
            current: spec.root_id(),
            spec: spec,

            depth: 0,
            height_off_current: 0,
        }
    }

    fn try_child(&self, typ: ValueType, key: Option<&PathEntry>) -> PathPosition {
        match self.height_off_current {
            0 => {
                let current = self.spec.get(self.current);

                let child_node_id = match key {
                    None => current.variant.child_root(),
                    Some(&PathEntry::Index(index)) => current.variant.child_index(index),
                    Some(&PathEntry::Key(ref name)) => current.variant.child_key(&name),
                };

                let child_node_match = match child_node_id {
                    Some(child_id) => {
                        let child = self.spec.get(child_id);
                        if child.variant.matches(typ) {
                            Some(child_id)
                        } else {
                            None
                        }
                    }
                    None => None,
                };

                PathPosition {
                    current: child_node_match,
                    parent: Some(self.current),
                }
            }
            _ => {
                PathPosition {
                    current: None,
                    parent: None,
                }
            }
        }
    }

    pub fn visit_terminal<'a>(&'a mut self,
                              typ: ValueType,
                              key: Option<&PathEntry>)
                              -> PathPosition {
        self.try_child(typ, key)
    }

    pub fn visit_key(&self) -> Option<NodeId> {
        if self.height_off_current == 0 {
            Some(self.current)
        } else {
            None
        }
    }

    pub fn enter_nonterminal<'a>(&mut self,
                                 typ: ValueType,
                                 key: Option<&PathEntry>)
                                 -> PathPosition {
        let resp = self.try_child(typ, key);

        match resp.current {
            Some(child_id) => {
                self.current = child_id;
                self.depth += 1;
            }
            None => {
                self.height_off_current += 1;
            }
        }

        resp
    }

    pub fn exit_nonterminal<'a>(&'a mut self) -> PathPosition {
        match self.height_off_current {
            0 => {
                let current_id = self.current;
                self.current = self.spec.get(current_id).parent.unwrap();
                self.depth -= 1;
                PathPosition {
                    current: Some(current_id),
                    parent: Some(self.current),
                }
            }
            1 => {
                self.height_off_current -= 1;
                PathPosition {
                    current: None,
                    parent: Some(self.current),
                }
            }
            _ => {
                self.height_off_current -= 1;
                PathPosition {
                    current: None,
                    parent: None,
                }
            }
        }
    }
}
