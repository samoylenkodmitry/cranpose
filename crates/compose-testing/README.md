# compose-testing

Testing utilities and harnesses for validating Compose-RS behaviour.

## Robot testing

`RobotApp` wraps a real Compose-RS application inside an `AppShell` that
uses the headless pixels renderer. It lets integration tests launch a
composable tree, drive pointer input (move/press/release/click), and read
the rendered scene to assert on text or geometry.

`WgpuRobotApp` drives the same WGPU renderer used in production desktop
applications while rendering into an offscreen texture. It supports the
same pointer automation and scene access as `RobotApp`, and it can also
capture rendered frames for future screenshot-style tests.

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

Example using the WGPU-backed robot and capturing a frame:

```rust
use compose_testing::robot::rect_center;
use compose_testing::wgpu_robot::WgpuRobotApp;
use desktop_app::app::combined_app;

static ROBOTO_REGULAR: &[u8] = include_bytes!("../../../assets/Roboto-Regular.ttf");

let mut robot = WgpuRobotApp::launch_with_fonts(1024, 768, &[ROBOTO_REGULAR], || {
    combined_app();
})?;
robot.pump_until_idle(30)?;

let snapshot = robot.snapshot();
let button_rect = snapshot.text_rects("Increment")[0].clone();
let (x, y) = rect_center(&button_rect);
robot.click(x, y)?;
robot.pump_until_idle(30)?;

let capture = robot.capture_frame()?;
assert_eq!(capture.rgba().len(), (capture.width * capture.height * 4) as usize);
```
