use crate::modifier::Modifier;
use compose_core::{Node, NodeId};
use std::cell::Cell;

#[derive(Clone, Default)]
pub struct TextNode {
    pub modifier: Modifier,
    pub text: String,
    parent: Cell<Option<NodeId>>,
}

impl TextNode {
    pub fn parent_id(&self) -> Option<NodeId> {
        self.parent.get()
    }
}

impl Node for TextNode {
    fn on_attached_to_parent(&mut self, parent: NodeId) {
        self.parent.set(Some(parent));
    }

    fn on_removed_from_parent(&mut self) {
        self.parent.set(None);
    }

    fn parent(&self) -> Option<NodeId> {
        self.parent.get()
    }
}
