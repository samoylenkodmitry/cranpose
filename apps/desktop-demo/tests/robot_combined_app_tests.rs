use compose_testing::robot::{rect_center, RobotApp};
use desktop_app::app::combined_app;

#[test]
fn robot_can_wrap_real_app_and_drive_counter() {
    let mut robot = RobotApp::launch(|| {
        combined_app();
    });

    robot.set_viewport(1024.0, 768.0);
    robot.pump_until_idle(20);

    let snapshot = robot.snapshot();
    assert!(
        snapshot
            .text_values()
            .any(|text| text.contains("Counter: 0")),
        "counter tab should start at zero",
    );

    let increment_rect = snapshot
        .text_rects("Increment")
        .first()
        .cloned()
        .expect("increment button text should be visible");
    let (x, y) = rect_center(&increment_rect);

    assert!(
        robot.move_pointer(x, y),
        "pointer should land on increment button"
    );
    assert!(
        robot.click(x, y),
        "click should dispatch to increment button"
    );

    robot.pump_until_idle(20);
    let updated = robot.snapshot();
    assert!(
        updated
            .text_values()
            .any(|text| text.contains("Counter: 1")),
        "counter text should update after click",
    );

    robot.close();
}
