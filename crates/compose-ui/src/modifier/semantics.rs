use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use compose_foundation::{
    ModifierNode, ModifierNodeElement, NodeCapabilities, SemanticsConfiguration,
    SemanticsNode as SemanticsNodeTrait,
};

pub struct SemanticsModifierNode {
    recorder: Rc<dyn Fn(&mut SemanticsConfiguration)>,
}

impl SemanticsModifierNode {
    pub fn new(recorder: Rc<dyn Fn(&mut SemanticsConfiguration)>) -> Self {
        Self { recorder }
    }
}

impl ModifierNode for SemanticsModifierNode {
    fn as_semantics_node(&self) -> Option<&dyn SemanticsNodeTrait> {
        Some(self)
    }

    fn as_semantics_node_mut(&mut self) -> Option<&mut dyn SemanticsNodeTrait> {
        Some(self)
    }
}

impl SemanticsNodeTrait for SemanticsModifierNode {
    fn merge_semantics(&self, config: &mut SemanticsConfiguration) {
        (self.recorder)(config);
    }
}

#[derive(Clone)]
pub struct SemanticsElement {
    recorder: Rc<dyn Fn(&mut SemanticsConfiguration)>,
}

impl SemanticsElement {
    pub fn new<F>(recorder: F) -> Self
    where
        F: Fn(&mut SemanticsConfiguration) + 'static,
    {
        Self {
            recorder: Rc::new(recorder),
        }
    }
}

impl fmt::Debug for SemanticsElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SemanticsElement")
    }
}

impl PartialEq for SemanticsElement {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.recorder, &other.recorder)
    }
}

impl Eq for SemanticsElement {}

impl Hash for SemanticsElement {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.recorder).hash(state);
    }
}

impl ModifierNodeElement for SemanticsElement {
    type Node = SemanticsModifierNode;

    fn create(&self) -> Self::Node {
        SemanticsModifierNode::new(self.recorder.clone())
    }

    fn update(&self, node: &mut Self::Node) {
        node.recorder = self.recorder.clone();
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::SEMANTICS
    }
}
