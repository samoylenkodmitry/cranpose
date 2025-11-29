//! Example robot tests demonstrating the testing framework
//!
//! These tests show how to use the robot testing framework to test
//! real Compose apps with interactions and validations.

#[cfg(test)]
mod robot_tests {
    use compose_macros::composable;
    use compose_testing::robot::{create_headless_robot_test, RobotTestRule};
    use compose_testing::robot_assertions::*;
    use compose_ui::prelude::*;

    /// Example: Test a simple button click
    #[test]
    fn test_button_click() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let click_count = Rc::new(RefCell::new(0));
        let click_count_clone = click_count.clone();

        let mut robot = create_headless_robot_test(800, 600, move || {
            let count = click_count_clone.clone();
            simple_button_app(count);
        });

        // Wait for initial render
        robot.wait_for_idle();

        // Click at the button position (assuming it's centered)
        robot.click_at(400.0, 300.0);

        // Verify the click was registered
        assert_eq!(*click_count.borrow(), 1, "Button should be clicked once");
    }

    /// Example: Test finding elements by text
    #[test]
    fn test_find_by_text() {
        let mut robot = create_headless_robot_test(800, 600, || {
            text_app();
        });

        // Find element by text
        let mut finder = robot.find_by_text("Hello");
        finder.assert_exists();

        // Try to find non-existent text
        let mut finder = robot.find_by_text("Goodbye");
        finder.assert_not_exists();
    }

    /// Example: Test drag gesture
    #[test]
    fn test_drag_gesture() {
        let mut robot = create_headless_robot_test(800, 600, || {
            draggable_app();
        });

        // Perform a drag
        robot.drag(100.0, 100.0, 300.0, 300.0);

        // Verify the app state after drag
        robot.wait_for_idle();
    }

    /// Example: Test viewport resize
    #[test]
    fn test_viewport_resize() {
        let mut robot = create_headless_robot_test(800, 600, || {
            responsive_app();
        });

        // Initial size
        assert_eq!(robot.viewport_size(), (800, 600));

        // Resize viewport
        robot.set_viewport(1024, 768);

        // Verify new size
        assert_eq!(robot.viewport_size(), (1024, 768));

        robot.wait_for_idle();
    }

    /// Example: Test getting all text on screen
    #[test]
    fn test_get_all_text() {
        let mut robot = create_headless_robot_test(800, 600, || {
            multi_text_app();
        });

        let all_text = robot.get_all_text();

        // Verify we can extract text
        // (This is a placeholder until text extraction is fully implemented)
        println!("All text: {:?}", all_text);
    }

    /// Example: Test getting all rects
    #[test]
    fn test_get_all_rects() {
        let mut robot = create_headless_robot_test(800, 600, || {
            layout_app();
        });

        let all_rects = robot.get_all_rects();

        // Verify we can extract rectangles
        println!("All rects: {} elements", all_rects.len());
    }

    /// Example: Test long press
    #[test]
    fn test_long_press() {
        let mut robot = create_headless_robot_test(800, 600, || {
            long_press_app();
        });

        // Find and long press an element
        let mut finder = robot.find_at_position(400.0, 300.0);
        finder.long_press();

        robot.wait_for_idle();
    }

    /// Example: Test multiple interactions
    #[test]
    fn test_complex_interaction_flow() {
        let mut robot = create_headless_robot_test(800, 600, || {
            complex_app();
        });

        // Step 1: Click first button
        robot.click_at(200.0, 300.0);
        robot.wait_for_idle();

        // Step 2: Verify something changed
        // (Would check for state changes in a real app)

        // Step 3: Perform drag
        robot.drag(100.0, 100.0, 200.0, 200.0);
        robot.wait_for_idle();

        // Step 4: Click second button
        robot.click_at(600.0, 300.0);
        robot.wait_for_idle();
    }

    /// Example: Test dump screen for debugging
    #[test]
    fn test_dump_screen() {
        let mut robot = create_headless_robot_test(800, 600, || {
            debug_app();
        });

        // Dump screen state (useful for debugging)
        robot.dump_screen();
    }

    // Helper composable functions for tests

    #[composable]
    fn simple_button_app(click_count: Rc<RefCell<i32>>) {
        Column(|| {
            Text("Click Me");
            Box(
                Modifier::empty()
                    .size(100.0, 50.0)
                    .clickable(move |_| {
                        *click_count.borrow_mut() += 1;
                    }),
                || {},
            );
        });
    }

    #[composable]
    fn text_app() {
        Column(|| {
            Text("Hello");
            Text("World");
        });
    }

    #[composable]
    fn draggable_app() {
        Box(Modifier::empty().fill_max_size(), || {
            Text("Drag me");
        });
    }

    #[composable]
    fn responsive_app() {
        Box(Modifier::empty().fill_max_size(), || {
            Text("Responsive");
        });
    }

    #[composable]
    fn multi_text_app() {
        Column(|| {
            Text("Line 1");
            Text("Line 2");
            Text("Line 3");
        });
    }

    #[composable]
    fn layout_app() {
        Column(|| {
            Box(Modifier::empty().size(100.0, 100.0), || {});
            Box(Modifier::empty().size(200.0, 50.0), || {});
        });
    }

    #[composable]
    fn long_press_app() {
        Box(
            Modifier::empty()
                .size(100.0, 100.0)
                .clickable(|_| {}),
            || {},
        );
    }

    #[composable]
    fn complex_app() {
        Row(|| {
            Box(
                Modifier::empty()
                    .size(100.0, 100.0)
                    .clickable(|_| {}),
                || {
                    Text("Button 1");
                },
            );
            Box(
                Modifier::empty()
                    .size(100.0, 100.0)
                    .clickable(|_| {}),
                || {
                    Text("Button 2");
                },
            );
        });
    }

    #[composable]
    fn debug_app() {
        Column(|| {
            Text("Debug");
            Box(Modifier::empty().size(50.0, 50.0), || {});
        });
    }
}
