//! Node coordinator system mirroring Jetpack Compose's NodeCoordinator pattern.
//!
//! Coordinators wrap modifier nodes and form a chain that drives measurement, placement,
//! drawing, and hit testing. Each LayoutModifierNode gets its own coordinator instance
//! that persists across recomposition, enabling proper state and invalidation tracking.

use compose_foundation::{LayoutModifierNode, ModifierNodeContext, NodeCapabilities};
use compose_ui_layout::{Constraints, Measurable, Placeable};
use compose_core::NodeId;
use std::cell::{RefCell, Cell};
use std::rc::Rc;

use crate::modifier::{Size, Point};
use crate::modifier::ModifierChainHandle;
use crate::layout::{MeasurePolicy, MeasureResult, LayoutNodeContext};

/// Identifies what type of coordinator this is for debugging and downcast purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatorKind {
    /// The innermost coordinator that wraps the layout node's intrinsic content.
    Inner,
    /// A coordinator wrapping a single layout modifier node.
    LayoutModifier,
}

/// Core coordinator trait that all coordinators implement.
///
/// Coordinators are chained together, with each one wrapping the next inner coordinator.
/// This forms a measurement and placement chain that mirrors the modifier chain.
pub trait NodeCoordinator: Measurable {
    /// Returns the kind of this coordinator.
    fn kind(&self) -> CoordinatorKind;

    /// Returns the measured size after a successful measure pass.
    fn measured_size(&self) -> Size;

    /// Returns the position of this coordinator relative to its parent.
    fn position(&self) -> Point;

    /// Sets the position of this coordinator relative to its parent.
    fn set_position(&mut self, position: Point);

    /// Performs placement, which may recursively trigger child placement.
    fn place(&mut self, x: f32, y: f32);
}

/// Coordinator that wraps a single LayoutModifierNode from the reconciled chain.
///
/// This is analogous to Jetpack Compose's LayoutModifierNodeCoordinator.
/// It delegates measurement to the wrapped node, passing the inner coordinator as the measurable.
pub struct LayoutModifierCoordinator {
    /// Reference to the modifier chain to access the reconciled node.
    chain_handle: ModifierChainHandle,
    /// The index of this node in the modifier chain.
    node_index: usize,
    /// The inner (wrapped) coordinator.
    wrapped: Box<dyn NodeCoordinator>,
    /// The measured size from the last measure pass.
    measured_size: Cell<Size>,
    /// Position relative to parent.
    position: Cell<Point>,
    /// Shared context for invalidation tracking.
    context: Rc<RefCell<LayoutNodeContext>>,
}

impl LayoutModifierCoordinator {
    /// Creates a new coordinator wrapping the specified node.
    pub fn new(
        chain_handle: ModifierChainHandle,
        node_index: usize,
        wrapped: Box<dyn NodeCoordinator>,
        context: Rc<RefCell<LayoutNodeContext>>,
    ) -> Self {
        Self {
            chain_handle,
            node_index,
            wrapped,
            measured_size: Cell::new(Size::ZERO),
            position: Cell::new(Point::default()),
            context,
        }
    }

    /// Returns the index of the wrapped node in the modifier chain.
    pub fn node_index(&self) -> usize {
        self.node_index
    }
}

impl NodeCoordinator for LayoutModifierCoordinator {
    fn kind(&self) -> CoordinatorKind {
        CoordinatorKind::LayoutModifier
    }

    fn measured_size(&self) -> Size {
        self.measured_size.get()
    }

    fn position(&self) -> Point {
        self.position.get()
    }

    fn set_position(&mut self, position: Point) {
        self.position.set(position);
    }

    fn place(&mut self, x: f32, y: f32) {
        self.set_position(Point { x, y });
        // TODO: Placement logic - propagate to wrapped
    }
}

impl Measurable for LayoutModifierCoordinator {
    fn measure(&self, constraints: Constraints) -> Box<dyn Placeable> {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;

        // Access the reconciled node from the chain and invoke its measure method
        let chain = self.chain_handle.chain();

        // Get node entry to access the node mutably
        let size = if let Some(entry_ref) = chain.node_ref_at(self.node_index) {
            if let Some(node) = entry_ref.node() {
                // Downcast to known types and call measure
                let any = node.as_any();
                if let Some(node) = any.downcast_ref::<PaddingNode>() {
                    let mut ctx = self.context.borrow_mut();
                    node.measure(&mut *ctx, self.wrapped.as_ref(), constraints)
                } else if let Some(node) = any.downcast_ref::<SizeNode>() {
                    let mut ctx = self.context.borrow_mut();
                    node.measure(&mut *ctx, self.wrapped.as_ref(), constraints)
                } else if let Some(node) = any.downcast_ref::<FillNode>() {
                    let mut ctx = self.context.borrow_mut();
                    node.measure(&mut *ctx, self.wrapped.as_ref(), constraints)
                } else if let Some(node) = any.downcast_ref::<OffsetNode>() {
                    let mut ctx = self.context.borrow_mut();
                    node.measure(&mut *ctx, self.wrapped.as_ref(), constraints)
                } else if let Some(node) = any.downcast_ref::<TextModifierNode>() {
                    let mut ctx = self.context.borrow_mut();
                    node.measure(&mut *ctx, self.wrapped.as_ref(), constraints)
                } else {
                    // Unknown node type - fall back to wrapped
                    let placeable = self.wrapped.measure(constraints);
                    Size {
                        width: placeable.width(),
                        height: placeable.height(),
                    }
                }
            } else {
                // No node at this index - fall back to wrapped
                let placeable = self.wrapped.measure(constraints);
                Size {
                    width: placeable.width(),
                    height: placeable.height(),
                }
            }
        } else {
            // Index out of bounds - fall back to wrapped
            let placeable = self.wrapped.measure(constraints);
            Size {
                width: placeable.width(),
                height: placeable.height(),
            }
        };

        self.measured_size.set(size);
        Box::new(CoordinatorPlaceable { size })
    }

    fn min_intrinsic_width(&self, height: f32) -> f32 {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;

        let chain = self.chain_handle.chain();

        if let Some(node) = chain.node::<PaddingNode>(self.node_index) {
            node.min_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<SizeNode>(self.node_index) {
            node.min_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<FillNode>(self.node_index) {
            node.min_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<OffsetNode>(self.node_index) {
            node.min_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<TextModifierNode>(self.node_index) {
            node.min_intrinsic_width(self.wrapped.as_ref(), height)
        } else {
            self.wrapped.min_intrinsic_width(height)
        }
    }

    fn max_intrinsic_width(&self, height: f32) -> f32 {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;

        let chain = self.chain_handle.chain();

        if let Some(node) = chain.node::<PaddingNode>(self.node_index) {
            node.max_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<SizeNode>(self.node_index) {
            node.max_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<FillNode>(self.node_index) {
            node.max_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<OffsetNode>(self.node_index) {
            node.max_intrinsic_width(self.wrapped.as_ref(), height)
        } else if let Some(node) = chain.node::<TextModifierNode>(self.node_index) {
            node.max_intrinsic_width(self.wrapped.as_ref(), height)
        } else {
            self.wrapped.max_intrinsic_width(height)
        }
    }

    fn min_intrinsic_height(&self, width: f32) -> f32 {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;

        let chain = self.chain_handle.chain();

        if let Some(node) = chain.node::<PaddingNode>(self.node_index) {
            node.min_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<SizeNode>(self.node_index) {
            node.min_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<FillNode>(self.node_index) {
            node.min_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<OffsetNode>(self.node_index) {
            node.min_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<TextModifierNode>(self.node_index) {
            node.min_intrinsic_height(self.wrapped.as_ref(), width)
        } else {
            self.wrapped.min_intrinsic_height(width)
        }
    }

    fn max_intrinsic_height(&self, width: f32) -> f32 {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;

        let chain = self.chain_handle.chain();

        if let Some(node) = chain.node::<PaddingNode>(self.node_index) {
            node.max_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<SizeNode>(self.node_index) {
            node.max_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<FillNode>(self.node_index) {
            node.max_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<OffsetNode>(self.node_index) {
            node.max_intrinsic_height(self.wrapped.as_ref(), width)
        } else if let Some(node) = chain.node::<TextModifierNode>(self.node_index) {
            node.max_intrinsic_height(self.wrapped.as_ref(), width)
        } else {
            self.wrapped.max_intrinsic_height(width)
        }
    }
}

/// Inner coordinator that wraps the layout node's intrinsic content (MeasurePolicy).
///
/// This is analogous to Jetpack Compose's InnerNodeCoordinator.
pub struct InnerCoordinator {
    /// The measure policy to execute.
    measure_policy: Rc<dyn MeasurePolicy>,
    /// Child measurables.
    measurables: Vec<Box<dyn Measurable>>,
    /// Measured size from last measure pass.
    measured_size: Cell<Size>,
    /// Position relative to parent.
    position: Cell<Point>,
    /// Cached measure result for placements.
    last_measure_result: RefCell<Option<MeasureResult>>,
}

impl InnerCoordinator {
    /// Creates a new inner coordinator with the given measure policy and children.
    pub fn new(
        measure_policy: Rc<dyn MeasurePolicy>,
        measurables: Vec<Box<dyn Measurable>>,
    ) -> Self {
        Self {
            measure_policy,
            measurables,
            measured_size: Cell::new(Size::ZERO),
            position: Cell::new(Point::default()),
            last_measure_result: RefCell::new(None),
        }
    }
}

impl NodeCoordinator for InnerCoordinator {
    fn kind(&self) -> CoordinatorKind {
        CoordinatorKind::Inner
    }

    fn measured_size(&self) -> Size {
        self.measured_size.get()
    }

    fn position(&self) -> Point {
        self.position.get()
    }

    fn set_position(&mut self, position: Point) {
        self.position.set(position);
    }

    fn place(&mut self, x: f32, y: f32) {
        self.set_position(Point { x, y });
        // Execute placements from the last measure result
        if let Some(result) = self.last_measure_result.borrow().as_ref() {
            for placement in &result.placements {
                // Place children - actual implementation would go here
                let _ = placement;
            }
        }
    }
}

impl Measurable for InnerCoordinator {
    fn measure(&self, constraints: Constraints) -> Box<dyn Placeable> {
        // Execute the measure policy
        let result = self.measure_policy.measure(&self.measurables, constraints);

        // Cache the result for placement
        *self.last_measure_result.borrow_mut() = Some(result.clone());

        // Store measured size
        let size = result.size;
        self.measured_size.set(size);

        Box::new(CoordinatorPlaceable { size })
    }

    fn min_intrinsic_width(&self, height: f32) -> f32 {
        self.measure_policy.min_intrinsic_width(&self.measurables, height)
    }

    fn max_intrinsic_width(&self, height: f32) -> f32 {
        self.measure_policy.max_intrinsic_width(&self.measurables, height)
    }

    fn min_intrinsic_height(&self, width: f32) -> f32 {
        self.measure_policy.min_intrinsic_height(&self.measurables, width)
    }

    fn max_intrinsic_height(&self, width: f32) -> f32 {
        self.measure_policy.max_intrinsic_height(&self.measurables, width)
    }
}

/// Placeable implementation for coordinators.
struct CoordinatorPlaceable {
    size: Size,
}

impl Placeable for CoordinatorPlaceable {
    fn width(&self) -> f32 {
        self.size.width
    }

    fn height(&self) -> f32 {
        self.size.height
    }

    fn place(&self, _x: f32, _y: f32) {
        // Placement is handled by the coordinator
    }

    fn node_id(&self) -> NodeId {
        NodeId::default()
    }
}
