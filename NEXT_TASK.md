# Modifier System Migration Tracker

## Status: Coordinator still rebuilds hardcoded nodes and flattens modifier data; parity with Jetpack Compose is not reached.

## Reality Check (modifier gaps)
- Layout coordinators downcast to built-ins and clone nodes from config snapshots; custom or stateful layout modifiers are skipped (`crates/compose-ui/src/layout/coordinator.rs:201-389`).
- Layout modifiers cannot emit placement or hit-test logic: the trait returns only `Size`, and coordinator `place` is a passthrough (`compose-foundation/src/modifier.rs:392-439`, `crates/compose-ui/src/layout/coordinator.rs:326-330`).
- `ModifierChainHandle::compute_resolved` still collapses layout data every update, losing ordering and per-node state across phases (`crates/compose-ui/src/modifier/chain.rs:188-224`).
- Draw/pointer/text slices coalesce to “last write wins” instead of ordered chaining; text pipeline is string-only and stateless (`crates/compose-ui/src/modifier/slices.rs:70-176`, `crates/compose-ui/src/layout/coordinator.rs:172-195`).

## Next Actions (open-protocol roadmap)
1) **Phase 1 – Generic extensibility**
   - Move the measurement proxy trait to a public module and add `LayoutModifierNode::create_measurement_proxy()` so nodes supply their own snapshot objects.
   - Implement proxy factories on all built-ins (padding/size/fill/offset/text) and delete `extract_measurement_proxy` plus reconstruction from `layout/coordinator.rs`.
   - Update `LayoutModifierCoordinator` to call node-provided proxies on the live chain; add an integration test for a user-defined layout modifier that adds width.

2) **Phase 2 – State fidelity**
   - Add a stateful measure test (modifier increments on each measure) to expose the current stateless rehydration.
   - Refactor proxies to snapshot live node state instead of calling `Node::new(config)`; ensure invalidations propagate via the shared `LayoutNodeContext`.

3) **Phase 3 – Text layout pipeline**
   - Introduce a `TextLayoutResult` (size/baseline/glyphs) and have `TextModifierNode` produce/cache it during measure.
   - Store cached layouts in modifier slices, and update renderers to consume them instead of recomputing or using `text_content` strings alone.

4) **Phase 4 – Input/focus maturity**
   - Implement focus requester/event/key input modifier nodes and route window events through the focused modifier chain (tab/arrow navigation, key dispatch bubbling).

## References
- Kotlin coordinator pipeline: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`
- Kotlin modifier surface: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/Modifier.kt`
- Text reference: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt` and `.../text/modifiers/TextStringSimpleNode.kt`
