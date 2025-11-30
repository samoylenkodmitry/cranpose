# Robot Testing with Real App

This guide explains how to use the robot testing framework with **real desktop apps** (actual windows, GPU rendering) for screenshot testing and visual validation.

## Overview

The robot testing framework has two modes:

1. **Headless Mode** (`robot` module) - Fast unit tests with mock rendering
2. **Real App Mode** (`robot_app` module) - Full E2E tests with actual windows

## Quick Start

### 1. Enable the Feature

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
compose-testing = { path = "../../crates/compose-testing", features = ["robot-app"] }

[features]
robot-app = ["compose-testing/robot-app"]
```

### 2. Write Tests

```rust
#[cfg(all(test, feature = "robot-app"))]
mod real_app_tests {
    use compose_testing::robot_app::RobotApp;
    use crate::app;

    #[test]
    #[ignore] // Requires display
    fn test_real_app() {
        // Launch REAL app with actual window
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        // Wait for initial render
        std::thread::sleep(std::time::Duration::from_secs(1));

        // Interact with the app
        robot.click(400.0, 300.0).unwrap();
        robot.wait_frames(10).unwrap();

        // Take screenshot
        robot.screenshot("test_output.png").unwrap();

        // Cleanup
        robot.shutdown().unwrap();
    }
}
```

### 3. Run Tests

```bash
# Requires a display/window system (X11, Wayland, macOS window server, etc.)
cargo test --package desktop-app --features robot-app -- --ignored

# Run specific test
cargo test --package desktop-app --features robot-app test_real_app -- --ignored --nocapture
```

## API Reference

### RobotApp::launch()

Launch a real app with a window:

```rust
let robot = RobotApp::launch(width, height, content);
```

### Interactions

```rust
// Click at position
robot.click(x, y)?;

// Move cursor
robot.move_to(x, y)?;

// Drag gesture
robot.drag(from_x, from_y, to_x, to_y)?;

// Resize window
robot.resize(width, height)?;
```

### Waiting & Synchronization

```rust
// Wait for N frames to render
robot.wait_frames(10)?;

// Or use thread sleep for time-based waits
std::thread::sleep(Duration::from_millis(500));
```

### Screenshots

```rust
// Take screenshot, returns (width, height)
let (w, h) = robot.screenshot("output.png")?;
println!("Screenshot: {}x{}", w, h);
```

### Cleanup

```rust
// Always shutdown when done
robot.shutdown()?;
```

## Example Tests

### Basic App Launch

```rust
#[test]
#[ignore]
fn test_launch() {
    let robot = RobotApp::launch(800, 600, || {
        my_app();
    });

    std::thread::sleep(Duration::from_secs(2));
    robot.screenshot("screenshots/launch.png").unwrap();
    robot.shutdown().unwrap();
}
```

### Button Click

```rust
#[test]
#[ignore]
fn test_button_click() {
    let robot = RobotApp::launch(800, 600, || my_app());

    // Screenshot before
    robot.screenshot("screenshots/before.png").unwrap();

    // Click button
    robot.click(150.0, 560.0).unwrap();
    robot.wait_frames(10).unwrap();

    // Screenshot after
    robot.screenshot("screenshots/after.png").unwrap();

    robot.shutdown().unwrap();
}
```

### Tab Navigation

```rust
#[test]
#[ignore]
fn test_tab_switching() {
    let robot = RobotApp::launch(800, 600, || my_app());

    // Click different tabs and screenshot each
    for (i, x) in [70.0, 240.0, 400.0, 600.0].iter().enumerate() {
        robot.click(*x, 50.0).unwrap();
        robot.wait_frames(10).unwrap();
        robot.screenshot(&format!("screenshots/tab_{}.png", i)).unwrap();
    }

    robot.shutdown().unwrap();
}
```

### Window Resize

```rust
#[test]
#[ignore]
fn test_responsive() {
    let robot = RobotApp::launch(800, 600, || my_app());

    // Test different screen sizes
    for (w, h) in [(800, 600), (1024, 768), (400, 800)] {
        robot.resize(w, h).unwrap();
        robot.wait_frames(10).unwrap();
        robot.screenshot(&format!("screenshots/size_{}x{}.png", w, h)).unwrap();
    }

    robot.shutdown().unwrap();
}
```

### Visual Regression

```rust
#[test]
#[ignore]
fn test_visual_regression() {
    let robot = RobotApp::launch(800, 600, || my_app());

    std::thread::sleep(Duration::from_millis(500));

    // Take current screenshot
    robot.screenshot("screenshots/current.png").unwrap();

    // In a real test, you would:
    // 1. Load baseline image
    // 2. Compare pixels
    // 3. Assert diff < threshold
    //
    // Example with imagediff crate:
    // let diff = compare_images("baseline.png", "current.png");
    // assert!(diff < 0.01, "Visual diff too large: {}", diff);

    robot.shutdown().unwrap();
}
```

## Screenshot Testing

### Directory Structure

```
your-project/
├── tests/
│   └── robot_real_app_test.rs
└── screenshots/
    ├── baselines/          # Known-good screenshots
    │   ├── launch.png
    │   ├── button_click.png
    │   └── ...
    └── current/            # Current test run
        ├── launch.png
        └── ...
```

### Baseline Workflow

1. **Generate baselines** (first time):
   ```bash
   cargo test --features robot-app -- --ignored
   # Screenshots saved to screenshots/
   # Review and copy to screenshots/baselines/
   ```

2. **Run regression tests**:
   ```bash
   cargo test --features robot-app -- --ignored
   # Compare screenshots/ with screenshots/baselines/
   ```

3. **Update baselines** (when intentional changes):
   ```bash
   cp screenshots/*.png screenshots/baselines/
   ```

## CI/CD Integration

### GitHub Actions

```yaml
name: Robot Tests

on: [push, pull_request]

jobs:
  robot-tests:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y xvfb libxkbcommon-dev libwayland-dev

      - name: Run robot tests with Xvfb
        run: |
          xvfb-run cargo test --features robot-app -- --ignored

      - name: Upload screenshots
        uses: actions/upload-artifact@v3
        if: failure()
        with:
          name: test-screenshots
          path: screenshots/
```

### Docker

```dockerfile
FROM rust:latest

# Install display dependencies
RUN apt-get update && apt-get install -y \
    xvfb \
    libxkbcommon-dev \
    libwayland-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Run tests with virtual display
CMD ["xvfb-run", "cargo", "test", "--features", "robot-app", "--", "--ignored"]
```

## Troubleshonings

### No Display Error

**Error**: `Failed to create window` or `No display found`

**Solution**: Tests require a display. Options:
- Run locally with GUI
- Use Xvfb on Linux: `xvfb-run cargo test ...`
- Use macOS window server
- Skip with `#[ignore]` and run manually

### Screenshots Not Saving

**Error**: Screenshots don't appear

**Solution**:
- Create `screenshots/` directory first
- Check file permissions
- Use absolute paths if needed

### Timing Issues

**Problem**: Tests are flaky due to timing

**Solution**:
```rust
// Increase wait time
robot.wait_frames(30)?; // Instead of 10

// Or add sleep
std::thread::sleep(Duration::from_secs(1));

// Wait for specific condition
for _ in 0..100 {
    robot.wait_frames(1)?;
    if condition_met() { break; }
}
```

## Best Practices

### 1. Always Cleanup

```rust
// Use Result and ? for automatic cleanup
fn test_helper() -> Result<(), String> {
    let robot = RobotApp::launch(800, 600, || my_app());

    // Test code...

    robot.shutdown() // Returns Result
}

#[test]
#[ignore]
fn test_something() {
    test_helper().unwrap();
}
```

### 2. Organize Screenshots

```rust
// Use subdirectories
robot.screenshot("screenshots/feature_a/case_1.png")?;
robot.screenshot("screenshots/feature_b/case_1.png")?;

// Include timestamp for debugging
let timestamp = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_secs();
robot.screenshot(&format!("screenshots/debug_{}.png", timestamp))?;
```

### 3. Consistent Timing

```rust
// Use wait_frames for consistency
robot.click(100.0, 100.0)?;
robot.wait_frames(10)?; // Deterministic

// Not thread::sleep (non-deterministic frame count)
```

### 4. Test Isolation

```rust
// Each test launches fresh app
#[test]
#[ignore]
fn test_1() {
    let robot = RobotApp::launch(800, 600, || my_app());
    // ...
    robot.shutdown()?;
}

#[test]
#[ignore]
fn test_2() {
    let robot = RobotApp::launch(800, 600, || my_app()); // Fresh instance
    // ...
    robot.shutdown()?;
}
```

## Comparison with Headless Mode

| Feature | Headless (`robot`) | Real App (`robot_app`) |
|---------|-------------------|----------------------|
| Speed | Fast (~ms) | Slower (~seconds) |
| Display Required | No | Yes |
| Screenshots | No | Yes |
| GPU Rendering | No | Yes |
| Visual Testing | No | Yes |
| CI/CD | Easy | Requires Xvfb |
| Use Case | Unit tests | E2E, visual regression |

## See Also

- [Robot Testing Guide](ROBOT_TESTING.md) - Headless mode documentation
- [Example Tests](../apps/desktop-demo/src/tests/robot_real_app_test.rs) - Real examples
- [WgpuRenderer](../crates/compose-render/wgpu/) - Rendering backend
