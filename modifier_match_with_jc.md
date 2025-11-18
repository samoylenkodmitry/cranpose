# Modifier Migration Reality Check

Concise snapshot of how the modifier system differs from Jetpack Compose and what must change next.

## Current Snapshot (modifier-specific)

- Modifier nodes, capability masks, and `ModifierKind::Combined` exist and reconcile into a `ModifierNodeChain` (e.g., `crates/compose-foundation/src/modifier.rs`).
- Layout measurement runs through a coordinator chain, but each coordinator rebuilds hardcoded nodes from config snapshots (`crates/compose-ui/src/layout/coordinator.rs:201-389`) instead of invoking the live modifier node.
- Layout modifiers only return `Size` (no placement or hit-test hooks); coordinator placement is a pass-through (`compose-foundation/src/modifier.rs:392-439`, `crates/compose-ui/src/layout/coordinator.rs:326-330`), so padding/offset are reapplied manually via `ResolvedModifiers`.
- `ModifierChainHandle::compute_resolved` still flattens layout data (padding/size/fill/offset) every update, losing ordering and state when phases read the snapshot (`crates/compose-ui/src/modifier/chain.rs:188-224`).
- `ModifierNodeSlices` collapse draw/pointer/text to “last write wins” instead of ordered chaining; background/shape/text content are reduced to a single slot (`crates/compose-ui/src/modifier/slices.rs:15-176`).
- Text modifier remains string-only and stateless; proxies clone a new `TextModifierNode` per measure (`crates/compose-ui/src/layout/coordinator.rs:172-195`) with monospaced measure/dummy draw/semantics.

## Mismatches vs Jetpack Compose

- **Stateless, closed coordinator**: Kotlin keeps a persistent link to each live `LayoutModifierNode` and calls it directly for measure/placement/alignment/lookahead (`/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt:37-224`). Rust downcasts to a hardcoded list, clones a fresh node, and skips unknown/custom nodes (`crates/compose-ui/src/layout/coordinator.rs:201-389`), so stateful/custom layout modifiers cannot participate.
- **No placement/draw/pointer hooks on layout modifiers**: Layout modifiers cannot emit placement blocks; `LayoutModifierCoordinator::place` just forwards (`crates/compose-ui/src/layout/coordinator.rs:326-330`), and the trait exposes no placement API (`compose-foundation/src/modifier.rs:392-439`). Kotlin’s `MeasureResult` carries placements and alignment lines from each layout modifier.
- **Flattened snapshots instead of ordered chaining**: Layout/draw/pointer/semantics still read collapsed snapshots (`modifier/chain.rs:188-224`, `modifier/slices.rs:70-176`), so modifier ordering is lost and multiple instances coalesce (e.g., only one background/shape/text survives).
- **Text pipeline mismatch**: No paragraph/style/cache; measurement proxies recreate text nodes from strings only (`layout/coordinator.rs:172-195`), diverging from Kotlin’s `TextStringSimpleNode` pipeline that caches layout and drives draw/semantics.

## Roadmap (integrates “open protocol” proposal)

1) **Generic extensibility**: Make measurement proxy creation an API on `LayoutModifierNode` (factory per node), expose the proxy trait publicly, implement for all built-ins, and delete the hardcoded `extract_measurement_proxy` plus reconstruction path in `LayoutModifierCoordinator`.
2) **State fidelity**: Proxies snapshot live node state instead of `Node::new(config)`; add a stateful measure test that would currently reset each pass.
3) **Layout-to-render parity**: Cache text layout results and plumb them into modifier slices/renderers to avoid double layout and missing glyph data.
4) **Input/focus maturity**: Add focus/key modifier nodes and dispatch that respects the modifier chain, mirroring Compose’s focus/key processing.

These steps should align Rust behavior with Kotlin’s live-node coordinator while keeping borrow checker workarounds explicit. 
