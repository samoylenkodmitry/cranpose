use crate::robot::{rect_center, RobotApp};
use compose_core::useState;
use compose_macros::composable;
use compose_ui::widgets::Button;
use compose_ui::{Column, ColumnSpec, Modifier, Text};

#[composable]
fn CounterApp() {
    let count = useState(|| 0i32);
    Column(Modifier::empty(), ColumnSpec::default(), move || {
        Text(
            format!("Count: {}", count.value()),
            Modifier::empty().padding(4.0),
        );
        let on_click = count;
        Button(
            Modifier::empty().padding(4.0),
            move || {
                let current = on_click.value();
                on_click.set_value(current + 1);
            },
            || {
                Text("Tap".to_string(), Modifier::empty().padding(2.0));
            },
        );
    });
}

#[test]
fn robot_can_click_and_read_scene() {
    let mut robot = RobotApp::launch(|| {
        CounterApp();
    });

    robot.pump_until_idle(10);
    let snapshot = robot.snapshot();
    assert!(snapshot.text_values().any(|text| text == "Count: 0"));

    let button_rect = snapshot
        .text_rects("Tap")
        .first()
        .cloned()
        .expect("button text rect should exist");
    let (x, y) = rect_center(&button_rect);

    assert!(robot.move_pointer(x, y), "pointer should hit button");
    assert!(robot.click(x, y), "click dispatch should succeed");

    robot.pump_until_idle(10);
    let updated = robot.snapshot();
    assert!(updated.text_values().any(|text| text == "Count: 1"));
    assert!(!updated.text_rects("Count: 1").is_empty());

    robot.close();
}

#[test]
fn robot_can_press_and_release_individually() {
    let mut robot = RobotApp::launch(|| {
        CounterApp();
    });
    robot.pump_until_idle(5);

    let snapshot = robot.snapshot();
    let tap_rect = snapshot
        .text_rects("Tap")
        .first()
        .cloned()
        .expect("button text rect should exist");
    let (x, y) = rect_center(&tap_rect);

    assert!(robot.press(x, y));
    robot.pump_until_idle(5);
    assert!(robot.release(x, y));
    robot.pump_until_idle(5);

    let updated = robot.snapshot();
    assert!(updated.text_values().any(|text| text == "Count: 1"));
}
