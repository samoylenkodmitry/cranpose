# Next Task: Capability Bitmasks & Targeted Modifier Invalidations

## Context
Layout/subcompose nodes now reconcile a `ModifierChainHandle` and read padding/weight/offset/graphics-layer data from `ResolvedModifiers`, but the chain still treats every modifier as if it were layout-affecting. Jetpack Compose relies on capability bitmasks (`NodeKind.kt`, `ModifierNodeChain.kt`) to know which phases (layout, draw, pointer input, semantics, modifier locals) need to run and to route `InvalidationKind` precisely. Without those masks, Compose-RS continues to invalidate full layout/draw passes even when only pointer nodes change, and we can’t yet expose filtered iterators for renderers or pointer dispatch. The immediate parity gap is wiring capability metadata through `ModifierNodeElement`, aggregating it per layout node, and ensuring invalidations/short-circuiting match Kotlin semantics.

## Goals
1. Teach every modifier node/element to expose a capability bitmask mirroring Kotlin’s `NodeKind` flags (layout, draw, pointer, semantics, modifier locals).
2. Aggregate masks inside `ModifierChainHandle`, `LayoutNode`, and `SubcomposeLayoutNode`, and store them so runtime subsystems can query `has_draw_nodes`, `has_pointer_input_nodes`, etc.
3. Route `InvalidationKind` through the aggregated masks so `LayoutNode::mark_needs_measure`, `mark_needs_layout`, and future draw/pointer invalidations trigger only when a relevant capability is present.
4. Add tests covering capability aggregation and invalidation routing, preventing regressions as more node-backed modifiers arrive.

## Suggested Steps
1. **Define bitmasks**  
   - Mirror `androidx.compose.ui.node.NodeKind` by adding a `NodeCapability` bitflag type (likely in `compose_foundation`) and extending `ModifierNodeElement::capabilities()` / `NodeCapabilities` to return the mask.  
   - Update existing elements (`PaddingElement`, `BackgroundElement`, `ClickableElement`, future pointer nodes) to declare the correct bits.
2. **Aggregate per chain**  
   - In `crates/compose-ui/src/modifier/chain.rs`, accumulate the capability mask while reconciling the chain and expose getters like `layout_kind_set`, `draw_kind_set`, `pointer_kind_set`.  
   - Thread these aggregates into `LayoutNode`/`SubcomposeLayoutNode` (e.g., new `capabilities` field) so measurement/render pipelines can branch early when a slice is empty.
3. **Wire invalidations**  
   - Extend `LayoutNode::sync_modifier_chain` (and the subcompose variant) to inspect the aggregated mask when draining `InvalidationKind`s: only call `mark_needs_measure` if layout bits are present, add placeholders for draw/pointer/semantics invalidations, and ensure repeated requests short-circuit.  
   - Follow Kotlin’s `ModifierNode.onAttach`/`onDetach` + `NodeCoordinator.requestUpdate` flow to keep lifecycle consistent.
4. **Tests & parity checks**  
   - Add unit tests in `crates/compose-ui/src/modifier/tests/` and/or `modifier_nodes_tests.rs` that register fake nodes with different capability combinations, mutate them, and assert only the expected flags toggle.  
   - Add integration coverage in `crates/compose-ui/src/layout/tests/layout_tests.rs` to verify layout nodes skip `mark_needs_measure` when only pointer-capable nodes invalidate.

## Definition of Done
- `ModifierNodeElement` / `ModifierNode` expose capability masks aligned with Jetpack Compose’s `NodeKind`.
- `ModifierChainHandle` and layout/subcompose nodes cache aggregated capability bits and expose helpers for future pointer/draw/semantics traversals.
- `LayoutNode::sync_modifier_chain` (and the subcompose equivalent) routes invalidations through the masks so only relevant phases mark dirty.
- Tests cover capability aggregation and invalidation routing, and `cargo test -p compose-ui` passes.
