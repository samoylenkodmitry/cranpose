# Next Task: Complete Node-Backed Migration & Remove ModifierState

## Context
With focus, semantics, pointer input, and draw all using delegate-aware modifier nodes, `ModifierState` is now the last legacy artifact preventing 1:1 parity with Jetpack Compose. Currently, layout-affecting modifiers (size, offset, weight, alignment) still write to cached `ModifierState` fields instead of creating proper `ModifierNode` elements. This creates a dual system where some modifiers are node-backed and others use legacy caching, making the resolved modifier computation hybrid instead of purely node-driven.

## Current State
**Node-backed modifiers (✅ Done):**
- `padding` - Uses `PaddingElement` with `LayoutModifierNode`
- `background` - Uses `BackgroundElement` with `DrawModifierNode`
- `draw*` - Uses draw cache elements
- `clipToBounds` - Uses `ClipToBoundsElement`
- `pointerInput` - Uses `PointerInputElement`
- `clickable` - Uses `ClickableElement`
- `focusTarget` - Uses `FocusTargetElement`
- `semantics` - Uses `SemanticsElement`

**Legacy ModifierState modifiers (❌ Need migration):**
- Size: `width`, `height`, `size`, `width_intrinsic`, `height_intrinsic`, `fill_max_*`, `required_size`
- Offset: `offset`, `absolute_offset`
- Weight: `weight`, `weight_with_fill`, `columnWeight`, `rowWeight`
- Alignment: `align`, `alignInBox`, `alignInColumn`, `alignInRow`

## Goals
1. **Migrate all remaining modifiers to use `ModifierNodeElement`** - Create proper element/node pairs for size, offset, weight, and alignment
2. **Remove `ModifierState` struct entirely** - Delete the legacy caching system
3. **Make resolved modifier computation purely node-driven** - `ModifierChainHandle::compute_resolved()` should read only from node chain
4. **Update all factory methods to use elements** - Replace `with_state` calls with `modifier_element()`
5. **Ensure 1:1 Kotlin parity** - Behavior must match `androidx.compose.ui.layout.*` modifiers

## Jetpack Compose Reference
Key files in `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/`:
- `layout/Size.kt` - Size modifiers (`width`, `height`, `size`, `fillMax*`, `requiredSize`)
- `layout/Offset.kt` - Offset modifiers (`offset`, `absoluteOffset`)
- `layout/RowColumnImpl.kt` - Weight implementation
- `layout/BoxScope.kt`, `layout/ColumnScope.kt`, `layout/RowScope.kt` - Alignment scopes

## Implementation Plan

### Phase 1: Create Layout Modifier Node Elements

#### Step 1.1: Size Modifiers
Create `compose-ui/src/modifier/size.rs`:
```rust
pub struct SizeElement {
    width: Option<f32>,
    height: Option<f32>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    min_height: Option<f32>,
    max_height: Option<f32>,
}

pub struct SizeNode {
    state: NodeState,
    // ... size constraints
}

impl LayoutModifierNode for SizeNode {
    fn measure(&mut self, context: &mut dyn ModifierNodeContext,
               measurable: &dyn Measurable, constraints: Constraints) -> Size {
        // Apply size constraints and measure
    }
}
```

#### Step 1.2: Offset Modifiers
Create `compose-ui/src/modifier/offset.rs`:
```rust
pub struct OffsetElement {
    x: f32,
    y: f32,
    rtl_aware: bool,
}

pub struct OffsetNode {
    state: NodeState,
    offset: Point,
}

impl LayoutModifierNode for OffsetNode {
    fn measure(&mut self, ...) -> Size {
        // Measure wrapped content and apply offset
    }
}
```

#### Step 1.3: Fill Modifiers
Create `compose-ui/src/modifier/fill.rs`:
```rust
pub struct FillElement {
    direction: FillDirection, // Width, Height, Both
    fraction: f32,
}

pub enum FillDirection {
    Width,
    Height,
    Both,
}

impl LayoutModifierNode for FillNode {
    fn measure(&mut self, ...) -> Size {
        // Expand to max available space * fraction
    }
}
```

#### Step 1.4: Weight & Alignment
These are scope-specific (used within Row/Column/Box), so they should:
- Create `WeightElement` that stores weight data
- Create `AlignmentElement` that stores alignment data
- Parent containers (Row/Column/Box) read these during layout via node traversal

### Phase 2: Update Modifier API Factories

Update `compose-ui/src/modifier/mod.rs`:
```rust
pub fn width(width: f32) -> Self {
    let element = SizeElement::width(width);
    Modifier::from_parts(vec![modifier_element(element)], ModifierState::default())
}

pub fn offset(x: f32, y: f32) -> Self {
    let element = OffsetElement::new(x, y, true);
    Modifier::from_parts(vec![modifier_element(element)], ModifierState::default())
}

pub fn fill_max_width_fraction(fraction: f32) -> Self {
    let element = FillElement::width(fraction.clamp(0.0, 1.0));
    Modifier::from_parts(vec![modifier_element(element)], ModifierState::default())
}
```

### Phase 3: Update Resolved Modifier Computation

In `compose-ui/src/modifier/chain.rs`, update `compute_resolved()`:
```rust
fn compute_resolved(&self) -> ResolvedModifiers {
    let mut resolved = ResolvedModifiers::default();

    // Read from modifier nodes ONLY
    self.chain.for_each_forward_matching(NodeCapabilities::LAYOUT, |node_ref| {
        if let Some(node) = node_ref.node() {
            // Extract size/offset/padding from LayoutModifierNode
            if let Some(size_node) = node.as_any().downcast_ref::<SizeNode>() {
                resolved.set_layout_properties(size_node.to_layout_properties());
            }
            if let Some(offset_node) = node.as_any().downcast_ref::<OffsetNode>() {
                resolved.set_offset(offset_node.offset());
            }
            // ... handle other node types
        }
    });

    resolved
}
```

### Phase 4: Remove ModifierState

1. Delete `ModifierState` struct from `compose-ui/src/modifier/mod.rs`
2. Remove `state: Rc<ModifierState>` field from `Modifier`
3. Delete `with_state()`, `from_update()`, `merge()` methods
4. Update `from_parts()` to not accept `ModifierState`
5. Remove `layout`, `offset`, `background`, `corner_shape` getters that read from state

### Phase 5: Testing & Verification

Create comprehensive tests in each new module:
```rust
// In size.rs tests
#[test]
fn size_node_applies_constraints() {
    let element = SizeElement::width(100.0);
    let mut node = element.create();
    // Test that measure() respects width constraint
}

#[test]
fn fill_max_expands_to_parent() {
    let element = FillElement::width(1.0);
    let mut node = element.create();
    // Test that node fills available space
}

// In offset.rs tests
#[test]
fn offset_shifts_content() {
    let element = OffsetElement::new(10.0, 20.0, true);
    let mut node = element.create();
    // Test that layout position is offset correctly
}
```

## Acceptance Criteria

### Must Have (Blocking 1:1 Parity)
- ✅ All `Modifier` factories create `ModifierNodeElement` instances
- ✅ No `with_state()` calls remain in modifier factories
- ✅ `ModifierState` struct deleted entirely
- ✅ `Modifier::resolved_modifiers()` reads exclusively from node chain
- ✅ `cargo test` passes with no regressions
- ✅ Size behavior matches `androidx.compose.ui.layout.Size.kt`
- ✅ Offset behavior matches `androidx.compose.ui.layout.Offset.kt`
- ✅ Weight/alignment behavior matches Row/Column implementations

### Should Have (Quality)
- ✅ Tests cover all size constraint combinations (min, max, fixed, fraction)
- ✅ Tests verify offset with RTL layout
- ✅ Tests check weight distribution in Row/Column contexts
- ✅ Inspector metadata preserved for all migrated modifiers
- ✅ Performance is same or better than ModifierState (avoid allocations)

### Nice to Have (Future)
- Animated size/offset modifiers using the new node system
- Layout transition support
- Advanced intrinsic measurement

## Migration Strategy

**Order of implementation (least to most complex):**
1. **Offset** - Simplest, just position shift
2. **Size (fixed)** - Fixed width/height constraints
3. **Fill** - Max size with fraction
4. **Size (intrinsic)** - Min/max intrinsic measurements
5. **Weight** - Requires parent container integration
6. **Alignment** - Scope-specific, needs Row/Column/Box updates

**Incremental validation:**
- After each modifier migration, run `cargo test` to ensure no regressions
- Keep `ModifierState` alive until ALL modifiers migrated
- Use feature flags if needed to enable new system incrementally

## Expected Outcomes

After completion:
- ✅ **100% node-backed**: Every modifier goes through `ModifierNodeElement` → `ModifierNode` → capability-filtered traversal
- ✅ **Zero legacy caching**: No dual system, no ModifierState
- ✅ **Kotlin parity**: Behavior matches Jetpack Compose layout modifiers 1:1
- ✅ **Cleaner API**: `Modifier` becomes a pure element builder with no hidden state
- ✅ **Better diagnostics**: All modifiers visible in node chain dumps
- ✅ **Foundation complete**: Ready for advanced features (animations, transitions, gestures)

## Cross-References
- Roadmap: `modifier_match_with_jc.md` - Migration Plan step 1, 4, 5
- Roadmap: Phase 1 (Stabilize resolved data), Near-Term Next Steps #3
- Prior art: `padding.rs`, `background.rs`, `focus.rs` for element/node patterns
- Kotlin reference: `/media/huge/composerepo/compose/ui/ui/src/commonMain/kotlin/androidx/compose/ui/layout/`

## Success Metrics
```rust
// After completion, this should work identically to Kotlin Compose:
Modifier::empty()
    .size(100.0, 100.0)
    .offset(10.0, 20.0)
    .padding(EdgeInsets::all(8.0))
    .background(Color::BLUE)
    .clip_to_bounds()
    .focusTarget()
    // All modifiers are nodes, all use capability filtering, zero legacy state
```

## Notes
- **No unsafe code** - All implementations must use safe Rust
- **Maintain API compatibility** - Public modifier methods keep same signatures
- **Follow Kotlin semantics** - When in doubt, check Android implementation
- **Document deviations** - Any intentional differences from Kotlin must be noted
