# Modifier System Migration Tracker

## Status: ✅ Core migration complete - coordinators use live nodes, rendering via modifier slices, text 100% through TextModifierNode, debug_chain API available

## Baseline (useful context)

- Modifier chain uses `ModifierKind::Combined`; reconciliation via `ModifierChainHandle` feeds a node
  chain with capability tracking.
- Widgets emit `LayoutNode` + `MeasurePolicy`; dispatch queues keep pointer/focus flags in sync.
- Built-in layout modifier nodes exist (padding/size/fill/offset/text).
- Layout measurement goes through a coordinator chain
  (`crates/compose-ui/src/layout/mod.rs:725`), but `LayoutModifierCoordinator` downcasts to
  built-ins and recreates them (`crates/compose-ui/src/layout/coordinator.rs:175`) instead of
  calling live nodes; placement is stubbed (`coordinator.rs:164`) and padding/offset is still
  accumulated separately as a workaround.

## Architecture Overview

- **Widgets**: Pure composables emitting `LayoutNode`s with policies.
- **Modifier chain**: Builders chain via `then`; reconciliation via `ModifierChainHandle` with capability tracking.
- **Measure pipeline**: ✅ Coordinator chain uses live modifier nodes via MeasurementProxy pattern; placement propagates through chain.
- **Text**: ✅ Text flows 100% through `TextModifierNode` (LayoutModifierNode + DrawModifierNode); `LayoutNodeKind::Text` removed.
- **Rendering**: ✅ Visual properties (background, graphics_layer, text) render via `modifier_slices`; `ResolvedModifiers` only contains layout data.
- **Invalidation**: Capability flags drive invalidations through modifier node chain.

## Jetpack Compose reference cues (what we’re missing)

- `LayoutModifierNodeCoordinator` keeps a persistent handle to the live modifier node and measures
  it directly instead of cloning (`.../LayoutModifierNodeCoordinator.kt:37-195`).
- The same coordinator drives placement/draw/alignment and wraps the next coordinator for ordering
  (`LayoutModifierNodeCoordinator.kt:240-280`), so per-node state and lookahead are preserved.
- Node capabilities are honored per phase; draw/pointer/semantics are dispatched through the node
  chain rather than flattened snapshots.

## Completed Gaps ✅

### ✅ Coordinator chain (Phase 1)
- Coordinator chain now uses live modifier nodes via `MeasurementProxy` pattern
- All layout modifiers (padding/size/fill/offset/text) measured through proxy system
- Placement propagates through coordinator chain (`place()` calls wrapped coordinator)
- Removed NodeKind downcasting - uses trait-based measurement

### ✅ Rendering pipeline (Phase 2)
- Visual properties (background, corner_shape, graphics_layer) migrated to `ModifierNodeSlices`
- Rendering via `modifier_slices.draw_commands` and `modifier_slices.text_content()`
- `ResolvedModifiers` cleaned to only contain layout properties (padding, layout, offset)
- Click handling deduplicated - single path through `modifier_slices.click_handlers`

### ✅ Text integration (Phase 3)
- Text flows 100% through `TextModifierNode` (no dual paths)
- `LayoutNodeKind::Text` enum variant removed completely
- Text nodes report as `LayoutNodeKind::Layout` with content in `modifier_slices`
- Renderers (pixels, wgpu) updated to use `modifier_slices.text_content()`

## Remaining Gaps

### Reconciliation flattening
- `ModifierChainHandle::update()` still flattens element vectors on each update
- However, `ModifierNodeChain` reconciles properly and reuses node state
- Not a correctness issue, reconciliation works as designed

### Integration testing
- Need HitTestTarget integration tests for pointer/focus/text paths
- Current tests validate functionality but not through full dispatch pipeline

## Completed Work ✅

### ✅ Phase 1: Core Architecture & Layout
- [x] Generalized coordinator construction using `MeasurementProxy` trait
- [x] Refactored `LayoutModifierCoordinator` to measure live nodes (no NodeKind snapshot)
- [x] Implemented placement propagation through coordinator chain
- Commits: 6cb2dd4, 9a47b93, e0ada41

### ✅ Phase 2: Rendering Pipeline & Visual Correctness
- [x] Deduplicated click handling via `modifier_slices.click_handlers`
- [x] Migrated background + corner_shape to DrawCommand::Behind
- [x] Removed visual properties from ResolvedModifiers (background, corner_shape, graphics_layer)
- [x] Rendering via modifier slices only
- Commits: 5b68841, 6cb2dd4

### ✅ Phase 3: Text Integration
- [x] Text renders via `modifier_slices.text_content()` from TextModifierNode
- [x] Removed `LayoutNodeKind::Text` enum variant completely
- [x] Updated pixels and wgpu renderers to use modifier slices for text
- [x] Fixed test expectations for text nodes as LayoutNodeKind::Layout
- Commit: 510df62

### ✅ Phase 4: Input & Focus Wiring (Partial)
- [x] Exposed `Modifier.debug_chain(tag)` API for modifier chain inspection
- [x] Pointer input already flows through nodes via `Modifier.pointer_input()`
- [x] FocusManager exists and is wired through capability system
- Commit: 653538b

### ✅ Phase 5: Standardization (Partial)
- [x] All modifiers use chainable pattern `Modifier::empty().foo()` (no static factories)
- [ ] Add integration tests for pointer/focus/text through `HitTestTarget`

## Remaining Work

### Optional: Integration Testing
- Add end-to-end tests that dispatch pointer events through HitTestTarget
- Add tests for focus traversal through FocusManager
- Add tests for text layout/draw/semantics through modifier node path

All 479 tests passing. Core modifier migration complete.

## References

- Kotlin modifier pipeline: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/Modifier.kt`
- Node coordinator chain: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/node/LayoutModifierNodeCoordinator.kt`
- Text reference: `/media/huge/composerepo/compose/foundation/foundation/src/commonMain/kotlin/androidx/compose/foundation/text/BasicText.kt`
  and `.../text/modifiers/TextStringSimpleNode.kt`
- Detailed parity checklist: [`modifier_match_with_jc.md`](modifier_match_with_jc.md)
