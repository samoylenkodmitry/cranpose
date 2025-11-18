# Modifier Migration Reality Check

Concise snapshot of how the modifier system differs from Jetpack Compose and what must change next.

## Current Snapshot (modifier-specific)

- **Coordinator Bypass**: `LayoutModifierCoordinator::measure` (`crates/compose-ui/src/layout/coordinator.rs:120`) explicitly checks for a measurement proxy. If `create_measurement_proxy()` returns `None`, the coordinator **skips the node's measure logic entirely** and falls back to measuring the wrapped content directly. This renders the `LayoutModifierNode::measure` default implementation useless for custom modifiers that don't supply a proxy.
- **Missing Placement API**: `LayoutModifierNode::measure` returns only `Size`. There is no mechanism to return a `MeasureResult` with a placement block (as in Kotlin). Consequently, `LayoutModifierCoordinator::place` is a hardcoded pass-through (`crates/compose-ui/src/layout/coordinator.rs:112`), preventing modifiers from affecting child placement (e.g., offset, alignment).
- **Flattened Resolution**: `ModifierChainHandle::compute_resolved` (`crates/compose-ui/src/modifier/chain.rs:188`) iterates the chain and flattens standard modifiers (Padding, Size, Offset) into a single `ResolvedModifiers` struct. This loses the interleaving of modifiers (e.g., `padding(10).background(...).padding(20)` becomes just 30 padding and one background).
- **Slice Coalescing**: `ModifierNodeSlices` (`crates/compose-ui/src/modifier/slices.rs`) collects draw commands and pointer inputs but reduces text content and graphics layers to "last write wins", preventing composition of these effects.
- **Proxy Dependency**: The system relies heavily on `MeasurementProxy` to avoid borrowing the modifier chain during measure. This is a divergence from Kotlin where the coordinator holds a reference to the live node.

## Mismatches vs Jetpack Compose

- **Live Node vs. Snapshot Proxy**: Kotlin's `LayoutModifierNodeCoordinator` calls `measure` on the live `LayoutModifierNode`. Rust's coordinator requires the node to produce a `MeasurementProxy` (a snapshot) to participate in measurement. If no proxy is produced, the node is ignored.
- **Placement Control**: Kotlin's `measure` returns a `MeasureResult` containing a `placeChildren` lambda. Rust's `measure` returns `Size` only, and placement is handled entirely by the coordinator's pass-through logic.
- **Chain Traversal**: Kotlin traverses the actual node chain for all operations. Rust flattens the chain into `ResolvedModifiers` for layout properties, losing the structural information necessary for correct order of operations in complex chains.

## Roadmap (integrates “open protocol” proposal)

1.  **Fix Layout Modifier Protocol**:
    - Change `LayoutModifierNode::measure` to return a `MeasureResult` (containing Size and a Placement trait/closure).
    - Update `LayoutModifierCoordinator` to execute this placement logic.
    - Remove the "no proxy = skip" behavior; the coordinator should call `measure` on the node (or a proxy of it) and respect the result.

2.  **Generic Extensibility**:
    - Ensure all built-in modifiers (Padding, Size, etc.) implement `LayoutModifierNode` correctly.
    - Deprecate/Remove `ResolvedModifiers` flattening in favor of the coordinator chain handling these properties via the `measure`/`place` protocol.

3.  **State Fidelity**:
    - Move towards using live nodes or more robust proxies that don't require full reconstruction on every frame.

4.  **Text & Input**:
    - Align text handling with the node system (TextModifierNode should participate in layout/draw properly, not just export a string). 
