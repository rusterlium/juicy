use std::collections::HashMap;
use rustler::types::atom::NifAtom;

mod from_term;
mod walker;

pub use self::from_term::spec_from_term;
pub use self::walker::{SpecWalker, PathEntry, PathPosition};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    Object,
    Array,
    String,
    Number,
    Boolean,
    Null,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub variant: NodeVariant,
    pub options: NodeOptions,
    pub parent: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeVariant {
    Sentinel,
    Any,
    Array { child: NodeId, },
    Map { child: NodeId, },
    MapKeys { children: HashMap<String, NodeId>, },
}

impl NodeVariant {

    pub fn matches(&self, value: ValueType) -> bool {
        match self {
            &NodeVariant::Sentinel => unreachable!(),
            &NodeVariant::Any => true,
            &NodeVariant::Map { .. } if value == ValueType::Object => true,
            &NodeVariant::MapKeys { .. } if value == ValueType::Object => true,
            &NodeVariant::Array { .. } if value == ValueType::Array => true,
            _ => false,
        }
    }

    pub fn child_root(&self) -> Option<NodeId> {
        if self == &NodeVariant::Sentinel {
            Some(NodeId(1))
        } else {
            unreachable!();
        }
    }

    pub fn child_key(&self, _key: &[u8]) -> Option<NodeId> {
        match self {
            &NodeVariant::Sentinel => unreachable!(),
            &NodeVariant::Any => None,
            &NodeVariant::Map { child } => Some(child),
            &NodeVariant::Array { .. } => None,
            _ => unimplemented!(),
        }
    }

    pub fn child_index(&self, _index: usize) -> Option<NodeId> {
        match self {
            &NodeVariant::Sentinel => unreachable!(),
            &NodeVariant::Any => None,
            &NodeVariant::Map { .. } => None,
            &NodeVariant::Array { child } => Some(child),
            _ => unimplemented!(),
        }
    }

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeOptions {
    pub stream: bool,
    pub stream_collect: bool,
    pub struct_atom: Option<NifAtom>,
    pub atom_mappings: Option<HashMap<Vec<u8>, NifAtom>>,
    pub ignore_non_atoms: bool,
}
impl Default for NodeOptions {
    fn default() -> Self {
        NodeOptions {
            stream: false,
            stream_collect: false,
            struct_atom: None,
            atom_mappings: None,
            ignore_non_atoms: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    nodes: Vec<Node>,
    root: NodeId,
}
impl Spec {
    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.0]
    }

    pub fn root_id(&self) -> NodeId {
        self.root
    }
}
