# Next Task: Delegate Stack Parity & Capability Propagation

## Context
Recent work removed all `unsafe` pointer paths from the modifier runtime: `ModifierNodeChain` now maintains sentinel-safe traversal helpers, `ModifierChainHandle` shares a `ModifierLocalsHandle`, and `LayoutNode` registers pointer-free metadata so ancestor modifier-local lookups mirror Kotlin’s behavior. The remaining blocker for behavioral parity is the lack of Kotlin’s delegate model. Our chain still reconciles a flat list of entries, so we cannot represent nested `Modifier.Node` delegates, propagate `aggregateChildKindSet` through delegate stacks, or honor the traversal contract defined in Jetpack Compose’s `DelegatableNode.kt` / `NodeChain.kt`. This prevents focus, modifier locals, pointer input, and semantics from short-circuiting based on delegate capabilities, and keyed reuse helpers (`padChain`, `trimChain`, delegate reuse) remain unimplemented.

Use the Kotlin sources under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui`—especially `Modifier.kt`, `ModifierNodeElement.kt`, `DelegatableNode.kt`, `DelegatingNode.kt`, and `NodeChain.kt`—as the reference for lifecycle and traversal semantics.

## Goals
1. Introduce a delegate-aware data model in `compose_foundation`: every `ModifierNode` should expose `node.parent`, `node.child`, and `aggregateChildKindSet` the same way Kotlin’s `Modifier.Node` does, including sentinel head/tail nodes.
2. Teach `ModifierNodeChain::update_from_slice` to build and reuse delegate stacks when a `ModifierNodeElement` produces `DelegatingNode`s, recomputing capability masks and honoring Kotlin’s diff helpers (`padChain`, `trimChain`, delegate reuse by key/hash).
3. Provide traversal helpers (`headToTail`, `visitAncestors`, capability-filtered walks) that operate on delegates rather than raw entries, and migrate runtime consumers (`ModifierChainHandle`, `ModifierLocalManager`, `LayoutNode::semantics_configuration`, pointer/focus utilities) to those helpers.
4. Add parity tests that mirror scenarios from `NodeChainTest.kt`/`DelegatableNodeTest.kt`: delegate stacking order, capability aggregation, invalidation propagation, and reuse/detach behavior.

## Suggested Steps
1. **Data model & traits**
   - Mirror `androidx.compose.ui.node.DelegatableNode` and `DelegatingNode`. Extend `ModifierNode` (or a new internal trait) with fields for `parent`, `child`, `kindSet`, and `aggregateChildKindSet`. Keep sentinel head/tail nodes compatible with delegates.
   - Provide safe APIs for `visitAncestors`, `visitChildren`, and `nearestAncestor(mask)` that match Kotlin’s behavior (see `DelegatableNode.kt`).

2. **Chain reconciliation**
   - When reconciling entries, let elements opt into delegation (e.g., via a `DelegatingNode` helper or builder). Build delegate stacks, reuse nodes based on key/hash, and update capability aggregates using the same rules as Kotlin’s `NodeChain.structuralUpdate`.
   - Port the lightweight diff helpers (`padChain`, `trimChain`) so keyed reuse behaves like Jetpack Compose when nodes move, duplicate, or disappear.

3. **Runtime consumers**
   - Update `ModifierNodeChain` traversal APIs (`for_each_forward_matching`, `visit_descendants_matching`, etc.) to operate on delegates and to short-circuit using `aggregateChildKindSet`.
   - Switch modifier locals, semantics extraction, pointer dispatch, and focus scaffolding to the new delegate-aware visitors. This should reduce full-chain scans when the required capability bit is absent.

4. **Testing & parity checks**
   - Add new tests under `crates/compose-foundation/src/tests/modifier_tests.rs` (or a dedicated module) that cover delegate creation, reuse, invalidation, and capability aggregation.
   - Include integration tests in `compose-ui` (modifier locals, semantics, pointer input) proving that delegate traversal short-circuits when `aggregateChildKindSet` is clear.

## Definition of Done
- `ModifierNodeChain` maintains delegate stacks that propagate parent/child links, `kindSet`, and `aggregateChildKindSet` exactly like Kotlin’s `NodeChain`.
- `ModifierChainHandle`, modifier locals, semantics, pointer input, and focus all consume the delegate-aware traversal helpers instead of reimplementing chain scans.
- Tests covering delegate stacking, reuse, invalidation, and capability short-circuiting pass (`cargo test -p compose-ui`), and the behavior matches the reference Kotlin sources.
