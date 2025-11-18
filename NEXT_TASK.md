# Modifier System Migration Tracker

## Status: Layout modifiers lack placement control and are bypassed if no proxy is provided.

## Reality Check (modifier gaps)
- **Coordinator Bypass**: `LayoutModifierCoordinator` skips the node's `measure` method if `create_measurement_proxy` returns `None`. This prevents custom layout modifiers from functioning unless they implement the proxy protocol.
- **No Placement API**: `LayoutModifierNode::measure` returns `Size`, not `MeasureResult`. There is no way for a modifier to influence the placement of its child (e.g., offset, alignment).
- **Flattening**: `ResolvedModifiers` flattens the chain, losing order and preventing correct interleaving of layout and draw modifiers.
- **Text**: Text is a second-class citizen, handled via string passing in slices rather than a full layout node.

## Proposed Solution

To achieve parity with Jetpack Compose, we must refactor the `LayoutModifierNode` protocol and the `LayoutModifierCoordinator` to support placement and eliminate the proxy bypass.

### 1. Layout Modifier Protocol Refactor
**Goal**: Allow layout modifiers to control placement and participate in the measure pass without proxies.

- **Update `LayoutModifierNode::measure` signature**:
  Change the return type from `Size` to `MeasureResult`.
  ```rust
  pub trait LayoutModifierNode: ModifierNode {
      fn measure(
          &self,
          context: &mut dyn ModifierNodeContext,
          measurable: &dyn Measurable,
          constraints: Constraints,
      ) -> MeasureResult; // Was Size
  }
  ```
- **Define `MeasureResult` for Modifiers**:
  Reuse or adapt `compose_ui_layout::MeasureResult`. It must hold the size and a placement closure/trait.
  Since Rust closures in structs are tricky with lifetimes, we might need a trait object or a specific `Placement` enum/struct.
  *Recommendation*: Use a boxed closure `Box<dyn FnOnce(&mut PlacementScope)>` or similar within the `MeasureResult`.

### 2. Coordinator Logic Update
**Goal**: Ensure `LayoutModifierCoordinator` always executes the node's measure logic.

- **Refactor `ModifierNodeChain` for Shared Ownership**:
  - **Problem**: Currently, `ModifierNodeChain` owns nodes uniquely (`Box`). Accessing a node requires borrowing the chain (and the `Applier`). This causes a borrow panic if we try to recursively measure (Coordinator A -> Node A -> Coordinator B -> Node B), because the chain is already borrowed for Node A.
  - **Solution**: Change `ModifierNodeChain` to store nodes as `Rc<RefCell<dyn ModifierNode>>`.
  - **Mechanism**:
    1.  Update `ModifierNodeEntry` to hold `Rc<RefCell<dyn ModifierNode>>`.
    2.  In `measure_through_modifier_chain`, collect `Rc` clones for all layout nodes *before* building the coordinator chain.
    3.  Pass `Rc<RefCell<dyn LayoutModifierNode>>` to `LayoutModifierCoordinator`.
    4.  `LayoutModifierCoordinator::measure` borrows its specific node from the `Rc` (safe, distinct `RefCell`) without touching the `Applier` or chain.

- **Remove Proxy Check**: Delete the `create_measurement_proxy` check in `LayoutModifierCoordinator::measure`.
- **Direct Measure Call**: The coordinator calls `node.borrow().measure(...)` directly.

### 3. Removing Flattening & ResolvedModifiers
**Goal**: Make Padding, Offset, etc., true layout modifiers.

- **Deprecate `ResolvedModifiers`**: Stop collecting padding/offset/size into this struct.
- **Migrate Built-ins**: Ensure `PaddingNode`, `OffsetNode`, `SizeNode` implement the new `LayoutModifierNode::measure` (returning `MeasureResult` with placement).
  - `PaddingNode`: Measure child with deflated constraints. Return size = child size + padding. Placement = offset child by top/left padding.
  - `OffsetNode`: Measure child. Return child size. Placement = place child at (x, y) offset.
- **Update Coordinator Chain**: The coordinator chain currently skips non-layout nodes. Ensure it includes *all* `LayoutModifierNode`s, including the newly migrated ones.

## Implementation Plan

### Phase 1: Shared Ownership & Protocol (The Foundation)
1.  **Refactor `ModifierNodeChain`**: Change storage to `Vec<Box<ModifierNodeEntry>>` where entry holds `Rc<RefCell<dyn ModifierNode>>`. Update all chain traversal methods to handle `RefCell`.
2.  **Modify `LayoutModifierNode`**: Update `measure` to return `MeasureResult`.
    - *Temporary*: Keep `create_measurement_proxy` but default it to `None` and mark deprecated.
3.  **Update `LayoutModifierCoordinator`**:
    - Update struct to hold `node: Rc<RefCell<dyn LayoutModifierNode>>`.
    - Implement `place` to execute the `MeasureResult`'s placement logic.
    - Refactor `measure` to call `self.node.borrow().measure(...)`.

### Phase 2: Migrating Standard Modifiers
1.  **PaddingNode**: Implement `measure` with placement.
2.  **OffsetNode**: Implement `measure` with placement.
3.  **SizeNode**: Already implements `measure`, update to return `MeasureResult` (placement is just (0,0)).
4.  **Remove `ResolvedModifiers` usage**: Update `LayoutNode` to stop using `resolved_modifiers.padding` and rely on the coordinator chain.

### Phase 3: Text & Input
1.  **TextModifierNode**: Implement as `LayoutModifierNode`. Measure = text layout. Place = draw text at (0,0).
2.  **Input**: Ensure input modifiers are properly interleaved in the chain (already mostly true, but verify hit testing respects placement offsets).

## Immediate Next Steps
1.  Create a reproduction test case where a custom layout modifier's placement logic is ignored.
2.  Prototype the `MeasureResult` change in a separate branch/module to verify borrow checker viability.

## References
- Kotlin coordinator pipeline: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`
- Kotlin modifier surface: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/Modifier.kt`
- Rust Coordinator: `crates/compose-ui/src/layout/coordinator.rs`
- Rust Modifier Chain: `crates/compose-ui/src/modifier/chain.rs`
