# Next Task: DelegatableNode & Capability Diff Parity

## Context
Modifier locals now match Kotlin’s behavior: providers/consumers stay registered per chain, ancestor resolution walks parent layout nodes, and invalidations bubble back through `LayoutNode`. The next blocker for 1:1 parity is the lack of Kotlin’s delegate-chain model. `ModifierNodeChain` reconciles a flat list of nodes, but it still cannot represent nested `Modifier.Node` delegates, propagate capability masks through delegates, or follow the `DelegatableNode` traversal contract Jetpack Compose relies on (`androidx/compose/ui/node/DelegatableNode.kt`, `NodeChain.kt`, `ModifierNodeElement.kt`). Without delegate awareness, focus, semantics, pointer, and modifier locals still walk whole chains instead of capability-scoped stacks, and keyed reuse rules (`trimChain`, `padChain`, delegate reuse) are only partially implemented.

## Goals
1. Introduce a `DelegatableNode`/`DelegatingNode` surface in `compose_foundation` so every `ModifierNode` can expose parent/child delegate links identical to Kotlin’s contract.
2. Update `ModifierNodeChain` reconciliation to build delegate stacks, recompute `aggregate_child_capabilities` with delegate data, and honor Kotlin’s diff helpers (`trimChain`, `padChain`, reuse by key/hash).
3. Expose traversal helpers (`head_to_tail`, `visit_ancestors`, capability-filtered delegate walks) that operate on delegates rather than raw entries, and make `ModifierChainHandle`/`LayoutNode` consume those helpers.
4. Add parity tests that cover delegate stacking, capability short-circuiting, and reuse semantics (mirroring scenarios from `NodeChainTest.kt` and `DelegatableNodeTest.kt` in the Kotlin tree).

## Suggested Steps
1. **Data model + traits**
   - Mirror `androidx/compose/ui/node/DelegatableNode` and `DelegatingNode`: add a trait (e.g., `DelegatableNode`) that every `ModifierNode` automatically implements, providing access to `node.parent`, `node.child`, `visitAncestors`, etc.
   - Store delegate metadata alongside each `ModifierNodeEntry` (parent/child pointers plus per-delegate capability masks). Keep sentinel head/tail entries compatible with delegates.
2. **Chain reconciliation + diffing**
   - When `ModifierNodeChain::update_from_slice` reuses or creates an entry, also rebuild the delegate stack produced by the element (Kotlin’s `DelegatingNode` uses `updateCoordinator`). Port the diff helpers (`trimChain`, `padChain`, delegate reuse rules) from `NodeChain.kt`.
   - Recompute `aggregate_child_capabilities` so delegates contribute their capability bits to ancestors, matching Kotlin’s `aggregateChildKindSet`.
3. **Traversal + runtime consumers**
   - Replace ad-hoc iterators (`draw_nodes`, `pointer_input_nodes`, modifier-local scans) with the new delegate-aware helpers. Ensure capability filters short-circuit based on delegate masks, not just entry masks.
   - Update `ModifierChainHandle`/`LayoutNode` to expose delegate-aware traversal to modifier locals, semantics, focus, and future subsystems.
4. **Tests + parity validation**
   - Add focused tests under `crates/compose-foundation/src/tests/modifier_tests.rs` (or new files) that mimic Kotlin’s `NodeChainTest`: verify delegate reuse, capability aggregation, traversal order, and invalidation propagation.
   - Include regression tests for capability short-circuiting (e.g., semantics visitor stops when no delegate advertises `SEMANTICS`).

## Definition of Done
- `compose_foundation` exposes a `DelegatableNode` contract equivalent to Kotlin’s API, and every `ModifierNode` automatically participates in delegate parent/child links.
- `ModifierNodeChain` builds delegate stacks during reconciliation, recomputes capability masks with delegate data, and honors Kotlin’s diff/reuse helpers.
- `ModifierChainHandle`/`LayoutNode` traverse nodes via the delegate helpers; pointer, semantics, modifier locals, and diagnostics no longer reimplement traversal.
- New tests document delegate stacking, capability aggregation, and traversal order; they pass along with `cargo test -p compose-ui`.
