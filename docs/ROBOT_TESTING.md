# Robot Testing Framework

The Robot Testing Framework provides a comprehensive solution for end-to-end testing of Compose-RS applications. It enables developers to write automated tests that launch real apps, perform user interactions, and validate UI state.

## Overview

The robot testing framework is inspired by UI testing frameworks like Espresso (Android) and XCUITest (iOS). It allows you to:

- **Launch your app** in a controlled testing environment
- **Perform interactions** such as clicks, drags, and gestures
- **Find UI elements** by text, position, or semantics
- **Validate UI state** including layout, text content, and visual properties
- **Test the full lifecycle** including animations and state changes

## Quick Start

### Basic Test Setup

```rust
use compose_testing::robot::create_headless_robot_test;
use compose_macros::composable;
use compose_ui::prelude::*;

#[test]
fn test_button_click() {
    use std::cell::RefCell;
    use std::rc::Rc;

    let clicked = Rc::new(RefCell::new(false));
    let clicked_clone = clicked.clone();

    let mut robot = create_headless_robot_test(800, 600, move || {
        let clicked = clicked_clone.clone();

        Column(|| {
            Text("Click the button");

            Box(
                Modifier::empty()
                    .size(100.0, 50.0)
                    .clickable(move |_| {
                        *clicked.borrow_mut() = true;
                    }),
                || {
                    Text("Click Me");
                }
            );
        });
    });

    // Wait for initial render
    robot.wait_for_idle();

    // Find and click the button
    robot.find_by_text("Click Me").click();

    // Verify the button was clicked
    assert!(*clicked.borrow(), "Button should be clicked");
}
```

## Core Components

### 1. RobotTestRule

The main entry point for robot tests. It wraps your app in a controlled testing environment.

```rust
use compose_testing::robot::create_headless_robot_test;

let mut robot = create_headless_robot_test(800, 600, || {
    my_app(); // Your composable app
});
```

**Key Methods:**

- `wait_for_idle()` - Wait for all compositions, layouts, and renders to complete
- `click_at(x, y)` - Perform a click at specific coordinates
- `move_to(x, y)` - Move the cursor to specific coordinates
- `drag(from_x, from_y, to_x, to_y)` - Perform a drag gesture
- `set_viewport(width, height)` - Resize the viewport
- `dump_screen()` - Print debug information about the current UI state

### 2. Element Finders

Find UI elements using various strategies:

```rust
// Find by text content
let mut finder = robot.find_by_text("Login");

// Find by position
let mut finder = robot.find_at_position(100.0, 200.0);

// Find clickable elements
let mut finder = robot.find_clickable();
```

**Finder Methods:**

- `exists()` - Check if the element exists
- `bounds()` - Get the element's bounding rectangle
- `center()` - Get the element's center point
- `width()` / `height()` - Get the element's dimensions
- `click()` - Click on the element
- `long_press()` - Perform a long press
- `assert_exists()` - Assert the element exists (panics if not)
- `assert_not_exists()` - Assert the element doesn't exist

### 3. Validation Methods

Validate the current UI state:

```rust
// Get all text on screen
let texts = robot.get_all_text();
assert!(texts.contains(&"Welcome".to_string()));

// Get all UI element bounds
let rects = robot.get_all_rects();
assert_eq!(rects.len(), 5, "Should have 5 elements");
```

### 4. Assertions

The framework provides assertion helpers for common validation patterns:

```rust
use compose_testing::robot_assertions::*;

// Approximate equality (useful for floating point comparisons)
assert_approx_eq(actual, expected, tolerance, "width should match");

// Rectangle assertions
assert_rect_approx_eq(actual_rect, expected_rect, tolerance, "bounds should match");
assert_rect_contains_point(rect, x, y, "point should be inside rect");

// Text assertions
assert_contains_text(&texts, "Hello", "should contain greeting");
assert_not_contains_text(&texts, "Error", "should not have errors");

// Count assertions
assert_count(&items, 3, "should have 3 items");
```

## Testing Patterns

### Pattern 1: Click and Validate

```rust
#[test]
fn test_counter_increment() {
    let mut robot = create_headless_robot_test(800, 600, counter_app);

    // Initial state
    robot.find_by_text("Count: 0").assert_exists();

    // Click increment button
    robot.find_by_text("+").click();
    robot.wait_for_idle();

    // Verify state changed
    robot.find_by_text("Count: 1").assert_exists();
}
```

### Pattern 2: Drag Interaction

```rust
#[test]
fn test_slider_drag() {
    let mut robot = create_headless_robot_test(800, 600, slider_app);

    // Drag slider from left to right
    robot.drag(100.0, 300.0, 400.0, 300.0);
    robot.wait_for_idle();

    // Verify slider value changed
    robot.find_by_text("50%").assert_exists();
}
```

### Pattern 3: Form Input

```rust
#[test]
fn test_login_flow() {
    let mut robot = create_headless_robot_test(800, 600, login_app);

    // Enter username
    robot.find_by_text("Username").click();
    // (Text input would require additional API)

    // Enter password
    robot.find_by_text("Password").click();

    // Submit form
    robot.find_by_text("Login").click();
    robot.wait_for_idle();

    // Verify login success
    robot.find_by_text("Welcome!").assert_exists();
}
```

### Pattern 4: Animation Testing

```rust
#[test]
fn test_fade_animation() {
    let mut robot = create_headless_robot_test(800, 600, fade_app);

    // Initial state
    robot.wait_for_idle();

    // Trigger animation
    robot.find_by_text("Animate").click();

    // Advance time
    robot.advance_time(1_000_000_000); // 1 second

    // Verify animation completed
    robot.find_by_text("Done").assert_exists();
}
```

### Pattern 5: Responsive Layout

```rust
#[test]
fn test_responsive_layout() {
    let mut robot = create_headless_robot_test(800, 600, responsive_app);

    // Desktop layout
    robot.find_by_text("Sidebar").assert_exists();

    // Resize to mobile
    robot.set_viewport(400, 800);
    robot.wait_for_idle();

    // Verify mobile layout
    robot.find_by_text("Menu").assert_exists();
    robot.find_by_text("Sidebar").assert_not_exists();
}
```

## Advanced Usage

### Custom Renderers

For integration tests with actual rendering, you can provide a custom renderer:

```rust
use compose_testing::robot::RobotTestRule;
use compose_render_wgpu::WgpuRenderer;

// Create with a real renderer (requires more setup)
let renderer = WgpuRenderer::new(/* ... */);
let mut robot = RobotTestRule::new(800, 600, renderer, my_app);
```

### Accessing the App Shell

For advanced scenarios, you can access the underlying `AppShell`:

```rust
let shell = robot.shell_mut();
// Direct access to shell methods
```

### Debugging Tests

When tests fail, use the debug utilities:

```rust
// Dump the entire screen state
robot.dump_screen();

// Get all text for inspection
let texts = robot.get_all_text();
println!("Current texts: {:?}", texts);

// Get all element bounds
let rects = robot.get_all_rects();
println!("Elements: {} rects", rects.len());
```

## Best Practices

### 1. Wait for Idle

Always call `wait_for_idle()` after interactions to ensure the UI has settled:

```rust
robot.click_at(100.0, 100.0);
robot.wait_for_idle(); // Wait for recomposition and layout
```

### 2. Use Semantic Finders

Prefer finding elements by text or semantics rather than positions:

```rust
// Good - resilient to layout changes
robot.find_by_text("Submit").click();

// Avoid - brittle if layout changes
robot.click_at(123.45, 678.90);
```

### 3. Use Assertions

Use the provided assertion helpers instead of raw assertions:

```rust
// Good - clear error messages
assert_approx_eq(width, 100.0, 1.0, "button width");

// Avoid - unclear errors
assert!((width - 100.0).abs() < 1.0);
```

### 4. Test One Thing

Each test should focus on a single behavior:

```rust
// Good - focused test
#[test]
fn test_button_increments_counter() {
    // Single responsibility
}

// Avoid - testing multiple things
#[test]
fn test_entire_app() {
    // Too broad
}
```

### 5. Use Descriptive Names

Name your tests clearly:

```rust
#[test]
fn test_clicking_increment_button_increases_counter_by_one() {
    // Clear what this tests
}
```

## Limitations

The current implementation has some limitations:

1. **Text Extraction**: Text extraction from the layout tree is not yet fully implemented. This affects `get_all_text()` and text-based finders.

2. **Semantics Queries**: Advanced semantic queries (finding by role, accessibility labels) are partially implemented.

3. **Text Input**: There's no API yet for simulating text input (keyboard typing).

4. **Multi-Touch**: Only single-pointer interactions are supported currently.

5. **Platform-Specific**: Some features work better on desktop than Android/mobile.

## Roadmap

Future improvements planned:

- [ ] Full text extraction from layout nodes
- [ ] Advanced semantic queries (by role, label, etc.)
- [ ] Text input simulation
- [ ] Multi-touch gesture support
- [ ] Screenshot capture for visual regression testing
- [ ] Performance profiling in tests
- [ ] Integration with test reporting tools

## Examples

See `apps/desktop-demo/src/tests/robot_test.rs` for comprehensive examples demonstrating all features of the robot testing framework.

## API Reference

For detailed API documentation, run:

```bash
cargo doc --package compose-testing --open
```

## Support

For questions or issues with the robot testing framework:

1. Check the examples in `apps/desktop-demo/src/tests/`
2. Read the API documentation
3. Open an issue on GitHub

## Contributing

Contributions to improve the robot testing framework are welcome! Areas that need work:

- Text extraction implementation
- Semantic query improvements
- Additional assertion helpers
- More example tests
- Documentation improvements
