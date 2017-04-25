use ::tree_spec::{SpecWalker, PathEntry, ValueType, PathPosition, NodeId};
use ::iterative_json_parser::{Position};

pub struct PathTracker {
    pub path: Vec<PathEntry>,
    pub walker: SpecWalker,
}

impl PathTracker {

    pub fn visit_terminal(&mut self, _pos: Position, typ: ValueType) -> PathPosition {
        let path_pos = self.walker.visit_terminal(typ, self.path.last());
        self.update_path();
        path_pos
    }

    pub fn enter_array(&mut self, _pos: Position) {
        {
            let last_key = self.path.last();
            self.walker.enter_nonterminal(ValueType::Array, last_key);
        }
        self.path.push(PathEntry::Index(0));
    }

    pub fn enter_map(&mut self, _pos: Position) {
        let last_key = self.path.last();
        self.walker.enter_nonterminal(ValueType::Object, last_key);
    }

    pub fn exit_array(&mut self) -> PathPosition {
        self.path.pop().unwrap();
        let path_pos = self.walker.exit_nonterminal();
        self.update_path();
        path_pos
    }

    pub fn exit_map(&mut self) -> PathPosition {
        let path_pos = self.walker.exit_nonterminal();
        self.update_path();
        path_pos
    }

    pub fn enter_key(&mut self, key: Vec<u8>) -> Option<NodeId> {
        self.path.push(PathEntry::Key(key));
        self.walker.visit_key()
    }

    fn update_path(&mut self) {
        match self.path.pop() {
            Some(PathEntry::Index(index)) => self.path.push(PathEntry::Index(index + 1)),
            Some(_) => (),
            None => (),
        }
    }

}
