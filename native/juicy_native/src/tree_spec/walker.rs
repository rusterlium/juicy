use super::{Spec, Node, NodeId, ValueType};

#[derive(Debug)]
pub enum PathEntry {
    Key(Vec<u8>),
    Index(usize),
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

impl SpecWalker {
    pub fn new(spec: Spec) -> SpecWalker {
        SpecWalker {
            current: spec.root_id(),
            spec: spec,

            depth: 0,
            height_off_current: 0,
        }
    }

    fn try_child(&self, typ: ValueType, key: Option<&PathEntry>) -> Option<NodeId> {
        if self.height_off_current == 0 {
            let current = self.spec.get(self.current);
            let child = match key {
                None => current.variant.child_root(),
                Some(&PathEntry::Index(index)) => current.variant.child_index(index),
                Some(&PathEntry::Key(ref name)) => current.variant.child_key(&name),
            };
            match child {
                Some(child_id) => {
                    let child = self.spec.get(child_id);
                    if child.variant.matches(typ) {
                        Some(child_id)
                    } else {
                        None
                    }
                },
                None => None,
            }
        } else {
            None
        }
    }

    pub fn visit_terminal<'a>(&'a mut self,
                              typ: ValueType,
                              key: Option<&PathEntry>)
                              -> Option<NodeId> {

        match self.try_child(typ, key) {
            Some(child_id) => Some(child_id),
            None => None,
        }
    }

    pub fn enter_nonterminal<'a>(&mut self,
                                 typ: ValueType,
                                 key: Option<&PathEntry>)
                                 -> Option<NodeId> {

        match self.try_child(typ, key) {
            Some(child_id) => {
                self.current = child_id;
                self.depth += 1;
                Some(child_id)
            },
            None => {
                self.height_off_current += 1;
                None
            },
        }
    }

    pub fn exit_nonterminal<'a>(&'a mut self) -> Option<NodeId> {
        if self.height_off_current == 0 {
            let current_id = self.current;
            self.current = self.spec.get(current_id).parent.unwrap();
            self.depth -= 1;
            Some(current_id)
        } else {
            self.height_off_current -= 1;
            None
        }
    }
}
