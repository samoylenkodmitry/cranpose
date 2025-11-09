# Next Task: Finish Modifier/NodeChain Parity With Jetpack Compose

## Context
`ModifierNodeChain` now owns safe head/tail sentinels, exposes parent/child links, and emits debug dumps via `COMPOSE_DEBUG_MODIFIERS`. Modifier locals and semantics have initial ports (`ModifierLocalKey`, provider/consumer nodes, `Modifier::semantics`, `LayoutNode::semantics_configuration`), and renderers/pointer input consume reconciled node slices. The outstanding parity work is to finish Kotlin’s delegate traversal contract, make modifier locals + semantics behave exactly like Android (including invalidations and ancestor lookups), and delete the remaining `ModifierState` responsibilities.

Our reference remains `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui`, especially:

- `node/NodeChain.kt`, `node/DelegatableNode.kt`, `node/NodeKind.kt`
- `modifier/ModifierLocal*`, `semantics/*`, `semantics/SemanticsNode.kt`
- `Modifier.kt` for inspector/debug dumps and chain tracing

## Goals
1. Complete delegate traversal parity: implement Kotlin’s `DelegatableNode` contract, delegate chains, and `aggregateChildKindSet` propagation so ancestor/descendant queries and invalidations match Android 1:1.
2. Finish modifier-local infrastructure: mirror `ModifierLocalManager`, ancestor lookups (walking parent layout nodes), and provider/consumer invalidations so modifier locals behave exactly like Kotlin’s `modifierLocalProvider/Consumer`.
3. Rebuild semantics on top of modifier nodes: port `SemanticsModifierNode`, `SemanticsOwner`, and tree extraction so `SemanticsTree` no longer relies on `RuntimeNodeMetadata`.
4. Expand diagnostics: add Kotlin-style `Modifier.toString()`, chain tracing (`NodeChain#trace` equivalent), and debug hooks that print delegate depth, capability masks, and modifier-local/semantics state.
5. Remove the remaining `ModifierState` caches after all public factories are node-backed and the runtime relies solely on reconciled node chains.

## Suggested Steps
1. **Delegate + traversal parity**  
   - Mirror `androidx.compose.ui.node.DelegatableNode`/`DelegatingNode`, including delegate chains, `node.parent/child`, and `aggregateChildKindSet` updates while diffing.  
   - Port Kotlin’s traversal helpers (`headToTail`, `tailToHead`, `visitAncestors`, `visitChildren`) and add exhaustive tests in `crates/compose-foundation/src/tests/modifier_tests.rs`.  
   - Ensure `LayoutNode` caches aggregate capability bits so ancestor lookups (semantics, modifier locals, focus) can short-circuit identical to Android.
2. **Modifier locals**  
   - Implement a `ModifierLocalManager` that tracks provider insert/remove/update events, mirroring `ModifierLocalManager.kt`. Hook it into `LayoutNode` so consumers invalidate when upstream values change.  
   - Update provider/consumer nodes to walk parent layout nodes when resolving values (use Kotlin’s `visitAncestors` semantics).  
   - Add parity tests based on `ModifierLocalTest.kt`, covering sibling/ancestor lookups, inter-layout propagation, and invalidation behavior.
3. **Semantics stack**  
   - Port `SemanticsModifierNode`, `SemanticsOwner`, and tree construction from Kotlin’s `semantics/*`. Replace `RuntimeNodeMetadata` semantics fields with direct traversal of modifier nodes.  
   - Wire semantics invalidations through modifier nodes and layout nodes so accessibility/focus layers can listen for changes.  
   - Mirror Android tests (e.g., clickable semantics, custom properties, merged configurations) under `crates/compose-ui/src/tests`.
4. **Diagnostics + tooling**  
   - Implement Kotlin-style `Modifier.toString()` / inspector strings (including delegate depth and modifier-local values), add `compose_ui::debug::trace_modifier_chain`, and surface capability masks per node.  
   - Ensure tracing respects `COMPOSE_DEBUG_MODIFIERS` and can be toggled per layout node for future focus/focusRequester debugging.
5. **ModifierState removal**  
   - Audit every modifier factory; migrate any remaining value-based helpers to node-backed implementations.  
   - Delete `ModifierState` caches once parity tests confirm layout/draw/inspector behavior, and update docs/examples to drop references to the legacy ModOp system.

## Definition of Done
- `ModifierNodeChain` exposes delegate traversal helpers and `aggregateChildKindSet` parity with Kotlin; ancestor/descendant queries and keyed diffing behave identically to `NodeChain.kt`.
- Modifier locals: providers/consumers, manager, invalidations, and ancestor lookups match Kotlin behavior, with tests covering intra-/inter-layout scenarios.
- Semantics: modifier nodes build the semantics tree (no `RuntimeNodeMetadata` fallback), expose parity actions/configurations, and pass new tests mirroring Android’s suite.
- Diagnostics: `Modifier::to_string()`/`Debug` output matches Kotlin’s formatting, `log_modifier_chain` (and new tracing helpers) print delegate/capability/modifier-local info, and can be toggled via env flags.
- `ModifierState` is removed from the runtime path; all public factories are node-backed, and renderers/layout rely solely on reconciled node chains. Workspace `cargo test` (plus targeted suites such as `cargo test -p compose-ui`) remains green.
