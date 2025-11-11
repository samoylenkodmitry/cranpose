# Next Task: Capability-Mask Dispatch & Targeted Invalidations

## Context
Diagnostics, inspector metadata, and per-node debug toggles now match Jetpack Compose’s `Modifier.inspectable` / `NodeChain#trace` behavior, so devtools can reason about individual chains again. The remaining behavioral gap is runtime dispatch: Compose-RS still leans on the legacy `as_*` downcasts and treats every draw invalidation as a layout dirty flag. Kotlin’s `NodeChain` routes work purely through capability masks (see `androidx/compose/ui/Modifier.kt`, `ModifierNodeElement.kt`, and `NodeChain.kt`) so DRAW-only mutations never force layout, pointer/focus traversals short-circuit before hitting unrelated nodes, and third-party nodes don’t need to override opt-in “shortcut” helpers.

## Current State
- `ModifierNodeChain::visit_*` exposes capability masks, but most runtime call sites (`draw_nodes`, pointer input, semantics, focus) still probe each entry with `as_*` helpers, meaning nodes must implement optional trait adapters to participate.
- `ModifierChainHandle::take_invalidations` only emits coarse `InvalidationKind`s; `LayoutNode::dispatch_modifier_invalidations` still calls `mark_needs_measure()` even when a draw-only node updates.
- Renderer and pointer stacks lack a `mark_needs_redraw()` / `mark_needs_pointer_repass()` path, so all invalidations bubble through layout dirtiness.
- Kotlin’s `NodeKind` masks allow delegation depth filtering (e.g., skip pointer nodes under a draw-only delegate) while we eagerly visit every node, even when the aggregate mask shows the capability is absent.

## Goals
1. **Mask-driven traversal** — Convert every runtime visitor (draw, pointer input, focus, semantics, modifier locals) to use `NodeCapabilities`/aggregate masks as the primary filter so nodes without `as_*` helpers still run.
2. **Targeted invalidation routing** — Track DRAW vs LAYOUT invalidations separately (e.g., `LayoutNode::mark_needs_redraw`) so draw-only changes no longer force measure/layout and renderer queues get precise dirties.
3. **Public surface polish** — Ensure modifier authors no longer implement `as_draw_node`/`as_pointer_input_node`; docs/tests should reference capabilities instead.

## Jetpack Compose Reference
- `androidx/compose/ui/Modifier.kt` (`Modifier.Element`) & `ModifierNodeElement.kt` for capability-driven dispatch contracts.
- `androidx/compose/ui/node/NodeChain.kt` and `DelegatableNode.kt` for mask-filtered traversal + invalidation routing.
- `androidx/compose/ui/platform/AndroidComposeView#invalidateLayers` for DRAW-only invalidation handling.

## Implementation Plan

### Phase 1 — Mask-Driven Visitors
1. Extend `ModifierNodeChain` helpers with ergonomics similar to Kotlin’s `visitAncestors/descendants` (delegate depth already exists).
2. Update pointer input, focus, semantics, draw, and modifier-local collectors to call `for_each_forward_matching(mask, …)` instead of `as_*` helpers.
3. Deprecate the `ModifierNode::as_*` shortcuts (leave no-op default impls for compat) and ensure built-in nodes rely solely on `NodeCapabilities`.
4. Tests: craft fake nodes that set `NodeCapabilities::DRAW` without overriding `as_draw_node` and assert draw traversal still hits them; same for pointer/focus.

### Phase 2 — Targeted Invalidation Routing
1. Teach `ModifierChainHandle`/`BasicModifierNodeContext` to record which capability triggered `InvalidationKind::Draw`, `::PointerInput`, etc.
2. Add `LayoutNode::mark_needs_redraw()` (and equivalent piping into `compose_ui::renderer`) so draw invalidations only enqueue render work; keep layout dirtiness untouched.
3. Ensure pointer/focus invalidations skip layout dirties entirely and instead notify their respective managers.
4. Tests: mutate a draw-only node and assert `needs_measure` stays false while render dirty flag flips; pointer/focus tests should assert no layout dirties occur.

### Phase 3 — Public Surface Cleanup
1. Remove documentation references to `as_draw_node`/friends and update examples to describe capability masks.
2. Audit `ModifierNodeElement::capabilities()` implementations to ensure every built-in node advertises the correct bits.
3. Provide a migration note (doc + changelog) telling downstream authors to set `NodeCapabilities` rather than override shortcut methods.
4. Tests: ensure `cargo doc` examples compile with the new API, and add regression tests covering third-party nodes that only set capability bits.

## Acceptance Criteria
- Every runtime traversal (draw, pointer, focus, semantics, modifier locals) depends on `NodeCapabilities` masks rather than optional `as_*` helpers.
- DRAW-only invalidations flow through a dedicated `mark_needs_redraw` path without toggling measure/layout flags.
- Pointer/focus invalidations no longer bubble through layout dirtiness.
- All built-in modifier nodes advertise accurate capability masks, and documentation reflects the new contract.
- `cargo test` (workspace) passes. 
