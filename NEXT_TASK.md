# Next Task: Wire Modifier Chains Into Real Layout Nodes

## Context
Modifier chains are now reconciled through `ModifierNodeChain` with full equality/hash semantics, but `LayoutNode`, `SubcomposeLayoutNode`, and the public layout primitives (Column/Row/Box) still read baked values from `ModifierState`. We need to push the reconciled chain + `ResolvedModifiers` through the layout tree so padding/offset/weight/graphics-layer data flows from modifier nodes the same way it does in Jetpack Compose. The Kotlin references (`LayoutNode.kt`, `NodeChain.kt`, Column/Row implementations) show how layout nodes own a chain handle, update it whenever their `Modifier` changes, and use capability-filtered iterators to drive measure/draw/subcompose pipelines.

## Goals
1. Add a `ModifierChainHandle` to `LayoutNode`/`SubcomposeLayoutNode` and update their `set_modifier` / `update` paths to reconcile nodes and cache `ResolvedModifiers`.
2. Teach Column/Row/Box (and any shared layout runner in `crates/compose-ui/src/layout`) to read padding/offset/weight from the reconciled handle instead of the legacy `ModifierState`.
3. Ensure invalidations emitted by modifier nodes propagate through `LayoutNode` so layout/draw passes re-run only when necessary.
4. Update docs/tests so future agents know layout primitives now depend on modifier nodes.

## Suggested Steps
1. **Embed the handle**
   - Update `crates/compose-ui/src/widgets/nodes/layout_node.rs` (and the subcompose variant) to own a `ModifierChainHandle`. When `modifier` changes, call `handle.update(modifier)` and store `handle.resolved_modifiers()`.
   - Expose lightweight accessors so measurement/draw code can ask a layout node for its resolved padding/background/weight info.
2. **Use resolved data in layouts**
   - In `crates/compose-ui/src/layout/mod.rs` (and any helper modules), replace direct reads of `ModifierState` padding/weight/offset with calls into the node handle. Start with Column/Row/Box since they are the highest-traffic primitives.
   - Ensure measurement order stays compatible with Kotlin: run layout modifier nodes first, then intrinsic/measure the children.
3. **Invalidations + plumbing**
   - When the handle reports layout/draw invalidations, propagate them through `LayoutNode::request_layout` / `request_draw` (or the local equivalents) so the runtime reruns the affected phases.
   - Add logging guarded by `COMPOSE_DEBUG_MODIFIERS` if it helps track regressions.
4. **Docs/tests**
   - Extend `modifier_match_with_jc.md` with a short note explaining layout nodes now reconcile modifier chains.
   - Add/adjust tests (e.g., in `crates/compose-ui/src/tests/layout_tests.rs`) verifying Column/Row respect node-provided padding/weight without touching `ModifierState`.

## Definition of Done
- Each layout node instance owns an up-to-date `ModifierChainHandle` and cached `ResolvedModifiers`.
- Column/Row/Box pull layout inputs (padding/weight/offset) from modifier nodes instead of `ModifierState`, and their behaviour matches Kotlin samples under `/media/huge/composerepo/.../compose/ui`.
- Modifier-node invalidations bubble through `LayoutNode` so layout/draw rerun when the chain requests it.
- Updated docs and tests cover the new plumbing, and `cargo test -p compose-ui` passes.
