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

use crate::modifier::{Size, Point, EdgeInsets};
use crate::layout::{MeasurePolicy, MeasureResult, LayoutNodeContext};
use crate::widgets::nodes::LayoutNode;

/// Snapshot of layout modifier node configuration to allow measurement without holding applier borrow.
enum NodeKind {
    Padding(EdgeInsets),
    Size {
        min_width: Option<f32>,
        max_width: Option<f32>,
        min_height: Option<f32>,
        max_height: Option<f32>,
        enforce: bool,
    },
    Fill {
        direction: crate::modifier_nodes::FillDirection,
        fraction: f32,
    },
    Offset {
        offset: Point,
        rtl_aware: bool,
    },
    Text(String),
}

impl NodeKind {
    fn measure(&self, context: &mut LayoutNodeContext, wrapped: &dyn Measurable, constraints: Constraints) -> Size {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;
        use compose_foundation::LayoutModifierNode;

        match self {
            NodeKind::Padding(padding) => {
                let node = PaddingNode::new(*padding);
                node.measure(context, wrapped, constraints)
            }
            NodeKind::Size { min_width, max_width, min_height, max_height, enforce } => {
                let node = SizeNode::new(*min_width, *max_width, *min_height, *max_height, *enforce);
                node.measure(context, wrapped, constraints)
            }
            NodeKind::Fill { direction, fraction } => {
                let node = FillNode::new(*direction, *fraction);
                node.measure(context, wrapped, constraints)
            }
            NodeKind::Offset { offset, rtl_aware } => {
                let node = OffsetNode::new(offset.x, offset.y, *rtl_aware);
                node.measure(context, wrapped, constraints)
            }
            NodeKind::Text(text) => {
                let node = TextModifierNode::new(text.clone());
                node.measure(context, wrapped, constraints)
            }
        }
    }
}

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
pub struct LayoutModifierCoordinator<'a> {
    /// Reference to the applier state to access the chain.
    state_rc: Rc<RefCell<crate::layout::LayoutBuilderState>>,
    /// The node ID to locate the LayoutNode.
    node_id: NodeId,
    /// The index of this node in the modifier chain.
    node_index: usize,
    /// The inner (wrapped) coordinator.
    wrapped: Box<dyn NodeCoordinator + 'a>,
    /// The measured size from the last measure pass.
    measured_size: Cell<Size>,
    /// Position relative to parent.
    position: Cell<Point>,
    /// Shared context for invalidation tracking.
    context: Rc<RefCell<LayoutNodeContext>>,
}

impl<'a> LayoutModifierCoordinator<'a> {
    /// Creates a new coordinator wrapping the specified node.
    pub fn new(
        state_rc: Rc<RefCell<crate::layout::LayoutBuilderState>>,
        node_id: NodeId,
        node_index: usize,
        wrapped: Box<dyn NodeCoordinator + 'a>,
        context: Rc<RefCell<LayoutNodeContext>>,
    ) -> Self {
        Self {
            state_rc,
            node_id,
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

impl<'a> NodeCoordinator for LayoutModifierCoordinator<'a> {
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
        // NOTE: Placement propagation not yet implemented.
        // In Jetpack Compose, this would call wrapped.place() with appropriate transformations.
        // Currently, placement is handled through MeasureResult placements after measurement.
        // This will be implemented when coordinators are extended to handle the full layout pass.
    }
}

impl<'a> Measurable for LayoutModifierCoordinator<'a> {
    fn measure(&self, constraints: Constraints) -> Box<dyn Placeable> {
        use crate::modifier_nodes::{PaddingNode, SizeNode, FillNode, OffsetNode};
        use crate::text_modifier_node::TextModifierNode;

        // Invoke the reconciled node's measure method. To avoid nested borrow conflicts
        // (where calling node.measure() tries to reborrow the applier that we're already
        // borrowing), we use the shared LayoutNodeContext directly and only borrow the
        // applier when needed within the node's measure implementation.
        //
        // For now, we handle known node types explicitly. When custom stateful LayoutModifierNodes
        // are added, this will need to be refactored to support dynamic dispatch without
        // nested borrows (possibly by changing coordinator ownership/lifetime model).

        let size = {
            let state = self.state_rc.borrow();
            let mut applier = state.applier.borrow_typed();

            let result = applier.with_node::<LayoutNode, _>(self.node_id, |layout_node| {
                let chain = layout_node.modifier_chain().chain();

                if let Some(entry_ref) = chain.node_ref_at(self.node_index) {
                    if let Some(node) = entry_ref.node() {
                        let any = node.as_any();

                        // Try to downcast to known types and invoke their measure
                        // The nodes delegate to wrapped.measure() which will eventually
                        // need to reborrow the applier, so we check first then release.
                        if let Some(padding_node) = any.downcast_ref::<PaddingNode>() {
                            let padding = padding_node.padding();
                            Some((NodeKind::Padding(padding), ()))
                        } else if let Some(size_node) = any.downcast_ref::<SizeNode>() {
                            let min_width = size_node.min_width();
                            let max_width = size_node.max_width();
                            let min_height = size_node.min_height();
                            let max_height = size_node.max_height();
                            let enforce = size_node.enforce_incoming();
                            Some((NodeKind::Size { min_width, max_width, min_height, max_height, enforce }, ()))
                        } else if let Some(fill_node) = any.downcast_ref::<FillNode>() {
                            Some((NodeKind::Fill { direction: fill_node.direction(), fraction: fill_node.fraction() }, ()))
                        } else if let Some(offset_node) = any.downcast_ref::<OffsetNode>() {
                            Some((NodeKind::Offset { offset: offset_node.offset(), rtl_aware: offset_node.rtl_aware() }, ()))
                        } else if let Some(text_node) = any.downcast_ref::<TextModifierNode>() {
                            Some((NodeKind::Text(text_node.text().to_string()), ()))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            });

            drop(applier);
            drop(state);

            // Now invoke measure with the extracted node info, applier borrow released
            match result {
                Ok(Some((node_kind, _))) => {
                    match self.context.try_borrow_mut() {
                        Ok(mut ctx) => node_kind.measure(&mut *ctx, self.wrapped.as_ref(), constraints),
                        Err(_) => {
                            let mut temp = LayoutNodeContext::new();
                            let size = node_kind.measure(&mut temp, self.wrapped.as_ref(), constraints);
                            if let Ok(mut shared) = self.context.try_borrow_mut() {
                                for kind in temp.take_invalidations() {
                                    shared.invalidate(kind);
                                }
                            }
                            size
                        }
                    }
                }
                _ => {
                    let placeable = self.wrapped.measure(constraints);
                    Size { width: placeable.width(), height: placeable.height() }
                }
            }
        };

        self.measured_size.set(size);
        Box::new(CoordinatorPlaceable { size })
    }

    fn min_intrinsic_width(&self, height: f32) -> f32 {
        let state = self.state_rc.borrow();
        let mut applier = state.applier.borrow_typed();

        applier
            .with_node::<LayoutNode, _>(self.node_id, |layout_node| {
                let chain = layout_node.modifier_chain().chain();

                if let Some(entry_ref) = chain.node_ref_at(self.node_index) {
                    if let Some(node) = entry_ref.node() {
                        if let Some(layout_modifier) = node.as_layout_node() {
                            return layout_modifier.min_intrinsic_width(self.wrapped.as_ref(), height);
                        }
                    }
                }
                self.wrapped.min_intrinsic_width(height)
            })
            .unwrap_or_else(|_| self.wrapped.min_intrinsic_width(height))
    }

    fn max_intrinsic_width(&self, height: f32) -> f32 {
        let state = self.state_rc.borrow();
        let mut applier = state.applier.borrow_typed();

        applier
            .with_node::<LayoutNode, _>(self.node_id, |layout_node| {
                let chain = layout_node.modifier_chain().chain();

                if let Some(entry_ref) = chain.node_ref_at(self.node_index) {
                    if let Some(node) = entry_ref.node() {
                        if let Some(layout_modifier) = node.as_layout_node() {
                            return layout_modifier.max_intrinsic_width(self.wrapped.as_ref(), height);
                        }
                    }
                }
                self.wrapped.max_intrinsic_width(height)
            })
            .unwrap_or_else(|_| self.wrapped.max_intrinsic_width(height))
    }

    fn min_intrinsic_height(&self, width: f32) -> f32 {
        let state = self.state_rc.borrow();
        let mut applier = state.applier.borrow_typed();

        applier
            .with_node::<LayoutNode, _>(self.node_id, |layout_node| {
                let chain = layout_node.modifier_chain().chain();

                if let Some(entry_ref) = chain.node_ref_at(self.node_index) {
                    if let Some(node) = entry_ref.node() {
                        if let Some(layout_modifier) = node.as_layout_node() {
                            return layout_modifier.min_intrinsic_height(self.wrapped.as_ref(), width);
                        }
                    }
                }
                self.wrapped.min_intrinsic_height(width)
            })
            .unwrap_or_else(|_| self.wrapped.min_intrinsic_height(width))
    }

    fn max_intrinsic_height(&self, width: f32) -> f32 {
        let state = self.state_rc.borrow();
        let mut applier = state.applier.borrow_typed();

        applier
            .with_node::<LayoutNode, _>(self.node_id, |layout_node| {
                let chain = layout_node.modifier_chain().chain();

                if let Some(entry_ref) = chain.node_ref_at(self.node_index) {
                    if let Some(node) = entry_ref.node() {
                        if let Some(layout_modifier) = node.as_layout_node() {
                            return layout_modifier.max_intrinsic_height(self.wrapped.as_ref(), width);
                        }
                    }
                }
                self.wrapped.max_intrinsic_height(width)
            })
            .unwrap_or_else(|_| self.wrapped.max_intrinsic_height(width))
    }
}

/// Inner coordinator that wraps the layout node's intrinsic content (MeasurePolicy).
///
/// This is analogous to Jetpack Compose's InnerNodeCoordinator.
pub struct InnerCoordinator<'a> {
    /// The measure policy to execute.
    measure_policy: Rc<dyn MeasurePolicy>,
    /// Child measurables.
    measurables: &'a [Box<dyn Measurable>],
    /// Measured size from last measure pass.
    measured_size: Cell<Size>,
    /// Position relative to parent.
    position: Cell<Point>,
    /// Shared result holder to store the measure result for placement.
    result_holder: Rc<RefCell<Option<MeasureResult>>>,
}

impl<'a> InnerCoordinator<'a> {
    /// Creates a new inner coordinator with the given measure policy and children.
    pub fn new(
        measure_policy: Rc<dyn MeasurePolicy>,
        measurables: &'a [Box<dyn Measurable>],
        result_holder: Rc<RefCell<Option<MeasureResult>>>,
    ) -> Self {
        Self {
            measure_policy,
            measurables,
            measured_size: Cell::new(Size::ZERO),
            position: Cell::new(Point::default()),
            result_holder,
        }
    }
}

impl<'a> NodeCoordinator for InnerCoordinator<'a> {
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
        if let Some(result) = self.result_holder.borrow().as_ref() {
            for placement in &result.placements {
                // Place children - actual implementation would go here
                let _ = placement;
            }
        }
    }
}

impl<'a> Measurable for InnerCoordinator<'a> {
    fn measure(&self, constraints: Constraints) -> Box<dyn Placeable> {
        // Execute the measure policy
        let result = self.measure_policy.measure(self.measurables, constraints);

        // Store measured size
        let size = result.size;
        self.measured_size.set(size);

        // Store the result in the shared holder for placement extraction
        *self.result_holder.borrow_mut() = Some(result);

        Box::new(CoordinatorPlaceable { size })
    }

    fn min_intrinsic_width(&self, height: f32) -> f32 {
        self.measure_policy.min_intrinsic_width(self.measurables, height)
    }

    fn max_intrinsic_width(&self, height: f32) -> f32 {
        self.measure_policy.max_intrinsic_width(self.measurables, height)
    }

    fn min_intrinsic_height(&self, width: f32) -> f32 {
        self.measure_policy.min_intrinsic_height(self.measurables, width)
    }

    fn max_intrinsic_height(&self, width: f32) -> f32 {
        self.measure_policy.max_intrinsic_height(self.measurables, width)
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
