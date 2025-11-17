//! Node coordinator system mirroring Jetpack Compose's NodeCoordinator pattern.
//!
//! Coordinators wrap modifier nodes and form a chain that drives measurement, placement,
//! drawing, and hit testing. Each LayoutModifierNode gets its own coordinator instance
//! that persists across recomposition, enabling proper state and invalidation tracking.

use compose_foundation::{InvalidationKind, LayoutModifierNode, ModifierNodeContext};
use compose_ui_layout::{Constraints, Measurable, Placeable};
use compose_core::NodeId;
use std::cell::RefCell;
use std::rc::Rc;

use crate::modifier::{Size, EdgeInsets, Point};

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

    /// Returns a reference to the wrapped (inner) coordinator, if any.
    fn wrapped(&self) -> Option<&dyn NodeCoordinator>;

    /// Returns a mutable reference to the wrapped (inner) coordinator, if any.
    fn wrapped_mut(&mut self) -> Option<&mut dyn NodeCoordinator>;

    /// Returns the measured size after a successful measure pass.
    fn measured_size(&self) -> Size;

    /// Returns the position of this coordinator relative to its parent.
    fn position(&self) -> Point;

    /// Sets the position of this coordinator relative to its parent.
    fn set_position(&mut self, position: Point);

    /// Performs placement, which may recursively trigger child placement.
    fn place(&mut self, x: f32, y: f32);

    /// Attempts to downcast to a LayoutModifierCoordinator for direct node access.
    fn as_layout_modifier_coordinator(&self) -> Option<&LayoutModifierCoordinator> {
        None
    }

    /// Attempts to downcast to a mutable LayoutModifierCoordinator.
    fn as_layout_modifier_coordinator_mut(&mut self) -> Option<&mut LayoutModifierCoordinator> {
        None
    }
}

/// Coordinator that wraps a single LayoutModifierNode.
///
/// This is analogous to Jetpack Compose's LayoutModifierNodeCoordinator.
/// It delegates measurement and intrinsics to the wrapped node, passing the
/// inner coordinator as the measurable.
pub struct LayoutModifierCoordinator {
    /// The layout modifier node this coordinator wraps.
    node_index: usize,
    /// The inner (wrapped) coordinator.
    wrapped: Box<dyn NodeCoordinator>,
    /// The measured size from the last measure pass.
    measured_size: Size,
    /// Position relative to parent.
    position: Point,
    /// The layout node ID for invalidation purposes.
    layout_node_id: NodeId,
}

impl LayoutModifierCoordinator {
    /// Creates a new coordinator wrapping the specified node.
    pub fn new(
        node_index: usize,
        wrapped: Box<dyn NodeCoordinator>,
        layout_node_id: NodeId,
    ) -> Self {
        Self {
            node_index,
            wrapped,
            measured_size: Size::ZERO,
            position: Point::default(),
            layout_node_id,
        }
    }

    /// Returns the index of the wrapped node in the modifier chain.
    pub fn node_index(&self) -> usize {
        self.node_index
    }

    /// Returns the layout node ID.
    pub fn layout_node_id(&self) -> NodeId {
        self.layout_node_id
    }
}

impl NodeCoordinator for LayoutModifierCoordinator {
    fn kind(&self) -> CoordinatorKind {
        CoordinatorKind::LayoutModifier
    }

    fn wrapped(&self) -> Option<&dyn NodeCoordinator> {
        Some(self.wrapped.as_ref())
    }

    fn wrapped_mut(&mut self) -> Option<&mut dyn NodeCoordinator> {
        Some(self.wrapped.as_mut())
    }

    fn measured_size(&self) -> Size {
        self.measured_size
    }

    fn position(&self) -> Point {
        self.position
    }

    fn set_position(&mut self, position: Point) {
        self.position = position;
    }

    fn place(&mut self, x: f32, y: f32) {
        self.set_position(Point { x, y });
        // Placement logic would go here - for now just propagate to wrapped
        if let Some(wrapped) = self.wrapped_mut() {
            wrapped.place(0.0, 0.0);
        }
    }

    fn as_layout_modifier_coordinator(&self) -> Option<&LayoutModifierCoordinator> {
        Some(self)
    }

    fn as_layout_modifier_coordinator_mut(&mut self) -> Option<&mut LayoutModifierCoordinator> {
        Some(self)
    }
}

impl Measurable for LayoutModifierCoordinator {
    fn measure(&self, constraints: Constraints) -> Box<dyn Placeable> {
        // This will be implemented to delegate to the actual node
        // For now, just delegate to wrapped
        self.wrapped().unwrap().measure(constraints)
    }

    fn min_intrinsic_width(&self, height: f32) -> f32 {
        self.wrapped().unwrap().min_intrinsic_width(height)
    }

    fn max_intrinsic_width(&self, height: f32) -> f32 {
        self.wrapped().unwrap().max_intrinsic_width(height)
    }

    fn min_intrinsic_height(&self, width: f32) -> f32 {
        self.wrapped().unwrap().min_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: f32) -> f32 {
        self.wrapped().unwrap().max_intrinsic_height(width)
    }
}

/// Inner coordinator that wraps the layout node's intrinsic content (MeasurePolicy).
///
/// This is analogous to Jetpack Compose's InnerNodeCoordinator.
pub struct InnerCoordinator {
    /// The measure policy to execute.
    measure_policy: Rc<dyn crate::layout::MeasurePolicy>,
    /// Child measurables.
    measurables: Vec<Box<dyn Measurable>>,
    /// Measured size from last measure pass.
    measured_size: Size,
    /// Position relative to parent.
    position: Point,
    /// Cached measure result for placements.
    last_measure_result: RefCell<Option<crate::layout::MeasureResult>>,
}

impl InnerCoordinator {
    /// Creates a new inner coordinator with the given measure policy and children.
    pub fn new(
        measure_policy: Rc<dyn crate::layout::MeasurePolicy>,
        measurables: Vec<Box<dyn Measurable>>,
    ) -> Self {
        Self {
            measure_policy,
            measurables,
            measured_size: Size::ZERO,
            position: Point::default(),
            last_measure_result: RefCell::new(None),
        }
    }
}

impl NodeCoordinator for InnerCoordinator {
    fn kind(&self) -> CoordinatorKind {
        CoordinatorKind::Inner
    }

    fn wrapped(&self) -> Option<&dyn NodeCoordinator> {
        None
    }

    fn wrapped_mut(&mut self) -> Option<&mut dyn NodeCoordinator> {
        None
    }

    fn measured_size(&self) -> Size {
        self.measured_size
    }

    fn position(&self) -> Point {
        self.position
    }

    fn set_position(&mut self, position: Point) {
        self.position = position;
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

        // Create a placeable that returns this coordinator's size
        let size = result.size;
        Box::new(SimplePlaceable { size })
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

/// Simple placeable implementation for returning sizes.
struct SimplePlaceable {
    size: Size,
}

impl Placeable for SimplePlaceable {
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
