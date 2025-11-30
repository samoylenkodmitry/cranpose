# compose-testing

Testing utilities and harnesses for validating Compose-RS behaviour.

## Robot testing

`RobotApp` wraps a real Compose-RS application inside an `AppShell` that
uses the headless pixels renderer. It lets integration tests launch a
composable tree, drive pointer input (move/press/release/click), and read
the rendered scene to assert on text or geometry.

Example usage against the `desktop-app` demo:

```rust
use compose_testing::robot::{rect_center, RobotApp};
use desktop_app::app::combined_app;

let mut robot = RobotApp::launch(|| {
    combined_app();
});
robot.set_viewport(1024.0, 768.0);
robot.pump_until_idle(20);

let button_rect = robot
    .snapshot()
    .text_rects("Increment")
    .first()
    .cloned()
    .expect("increment text should exist");
let (x, y) = rect_center(&button_rect);
robot.click(x, y);
robot.pump_until_idle(20);

assert!(
    robot
        .snapshot()
        .text_values()
        .any(|text| text.contains("Counter: 1"))
);
```
