# Modifier ≈ Jetpack Compose Parity Plan

Goal: match Jetpack Compose’s `Modifier` API surface and `Modifier.Node` runtime semantics so Kotlin samples and mental models apply 1:1 in Compose-RS.

---

## Current Gaps (Compose-RS)
- `Modifier` is still an Rc-backed builder with cached layout/draw state for legacy APIs. Renderers now read reconciled node slices, but `ModifierState` continues to provide padding/layout caches and must be removed once every factory is node-backed.
- `compose_foundation::ModifierNodeChain` now owns safe sentinel head/tail nodes plus parent/child metadata, yet it still lacks Kotlin’s delegate-chain surface (`DelegatableNode`, `NodeChain#headToTail`, `aggregateChildKindSet` propagation into layout nodes). Capability filtering remains per-chain rather than per ancestor traversal, so focus/modifier-local semantics can drift from Android.
- Modifier locals and semantics have an initial port (`ModifierLocalKey`, provider/consumer nodes, `Modifier::semantics`, `LayoutNode::semantics_configuration`), but invalidation, ancestor lookup, and semantics tree construction still rely on ad-hoc metadata instead of Kotlin’s `ModifierLocalManager`/`SemanticsOwner`.
- Diagnostics exist (`Modifier::fmt`, `debug::log_modifier_chain`, `COMPOSE_DEBUG_MODIFIERS`), but we still lack parity tooling such as Kotlin’s inspector strings, capability dumps with delegate depth, and targeted tracing hooks used by focus/pointer stacks.

## Jetpack Compose Reference Anchors
- `Modifier.kt`: immutable interface (`EmptyModifier`, `CombinedModifier`) plus `foldIn`, `foldOut`, `any`, `all`, `then`.
- `ModifierNodeElement.kt`: node-backed elements with `create`/`update`/`key`/`equals`/`hashCode`/inspector hooks.
- `NodeChain.kt`, `DelegatableNode.kt`, `NodeKind.kt`: sentinel-based chain, capability masks, delegate links, targeted invalidations, and traversal helpers.
- Pointer input stack under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/input/pointer`.

## Recent Progress
- `ModifierNodeChain` now stores safe sentinel head/tail nodes, parent/child links, and aggregate capability masks without `unsafe`, enabling deterministic traversal order and `COMPOSE_DEBUG_MODIFIERS` dumps.
- Modifier locals landed (`ModifierLocalKey`, provider/consumer elements, runtime manager sync), semantics nodes can be defined via `Modifier::semantics`, and `LayoutNode` can surface aggregated semantics configurations.
- Diagnostics improved: `Modifier` implements `Display`, `compose_ui::debug::log_modifier_chain` enumerates nodes/capabilities, and DEBUG env flags print chains after reconciliation.
- Core modifier factories (`padding`, `background`, `draw*`, `clipToBounds`, `pointerInput`, `clickable`) are node-backed, and pointer input runs on coroutine-driven scaffolding mirroring Kotlin. Renderers and pointer dispatch now operate exclusively on reconciled node slices.

## Migration Plan
1. **Mirror the `Modifier` data model (Kotlin: `Modifier.kt`)**  
   Keep the fluent API identical (fold helpers, `any`/`all`, inspector metadata) and delete the remaining runtime responsibilities of `ModifierState` once all factories are node-backed.
2. **Adopt `ModifierNodeElement` / `Modifier.Node` parity (Kotlin: `ModifierNodeElement.kt`)**  
   Implement the full lifecycle contract: `onAttach`, `onDetach`, `onReset`, coroutine scope ownership, and equality/key-driven reuse.
3. **Implement delegate traversal + capability plumbing (Kotlin: `NodeChain.kt`, `NodeKind.kt`, `DelegatableNode.kt`)**  
   Finish delegate links, ancestor aggregation, and traversal helpers so semantics/focus/modifier locals can walk nodes exactly like Android.
4. **Wire all runtime subsystems through chains**  
   Layout/draw/pointer already read reconciled nodes; remaining work includes semantics tree extraction, modifier locals invalidation, focus chains, and removal of the residual `ModifierState` caches.
5. **Migrate modifier factories + diagnostics**  
   Finish porting the remaining factories off `ModifierState`, add Kotlin-style inspector dumps/trace hooks, and grow the parity test matrix to compare traversal order/capabilities against the Android reference.

## Near-Term Next Steps
1. **Delegate + ancestor traversal parity**  
   - Port Kotlin’s delegate APIs (`DelegatableNode`, `DelegatingNode`, `node.parent/child` contract) so every `ModifierNode` exposes the same traversal surface as Android.  
   - Propagate `aggregateChildKindSet` (capability bitmasks) from nodes into `LayoutNode` so ancestor/descendant queries short-circuit exactly like `NodeChain.kt`.  
   - Mirror Kotlin’s `NodeChain` head/tail iteration helpers (`headToTail`, `tailToHead`, `trimChain/padChain`) for diffing, ensuring keyed reuse + capability recompute follow the reference semantics.
2. **Modifier locals parity**  
   - Flesh out a `ModifierLocalManager` that registers providers/consumers, invalidates descendants on insert/remove, and mirrors Kotlin’s `ModifierLocalConsumer` contract.  
   - Implement ancestor lookups that walk parent layout nodes (not just the current chain) and add parity tests based on `ModifierLocalTest.kt`.  
   - Connect modifier-local invalidations into `LayoutNode` dirty flags so layout/draw updates fire exactly as on Android.
3. **Semantics stack parity**  
   - Replace `RuntimeNodeMetadata` semantics fields with direct traversal of modifier nodes, build a `SemanticsOwner`/`SemanticsTree` identical to Kotlin’s implementation, and add parity tests (clickable semantics, content descriptions, custom actions).  
   - Wire semantics invalidations through modifier nodes + layout nodes, and feed the resulting semantics tree into accessibility/focus layers once available.
4. **Diagnostics + focus-ready infrastructure**  
   - Extend debugging helpers (`Modifier.toString()`, chain dumps) to include delegate depth, modifier locals provided, semantics flags, and capability masks.  
   - Port Kotlin’s snapshot tests/logging (`trace`, `NodeChain#trace`, `Modifier.toString()`) to prevent regressions once focus/focusRequester stacks land.
5. **Modifier factory + `ModifierState` removal**  
   - Audit every `Modifier` factory to ensure it’s fully node-backed; delete `ModifierState` caches after verifying layout/draw/inspection behavior via tests.  
   - Update docs/examples to emphasize node-backed factories and remove stale ModOp/`ModifierState` guidance.

## Kotlin Reference Playbook
| Area | Kotlin Source | Compose-RS Target |
| --- | --- | --- |
| Modifier API | `androidx/compose/ui/Modifier.kt` | `crates/compose-ui/src/modifier/mod.rs` |
| Node elements & lifecycle | `ModifierNodeElement.kt`, `DelegatableNode.kt` | `crates/compose-foundation/src/modifier.rs` + `compose-ui` node impls |
| Node chain diffing | `NodeChain.kt`, `NodeCoordinator.kt` | `crates/compose-foundation/src/modifier.rs`, upcoming coordinator module |
| Pointer input | `input/pointer/*` | `crates/compose-ui/src/modifier/pointer_input.rs` |
| Semantics | `semantics/*`, `SemanticsNode.kt` | `crates/compose-ui/src/semantics` (to be ported) |

Always cross-check behavior against the Kotlin sources under `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui` to ensure parity.
