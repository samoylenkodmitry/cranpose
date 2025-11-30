//! Robot tests with REAL app rendering
//!
//! These tests launch the actual desktop app with a real window and GPU rendering.
//! Useful for screenshot testing, visual regression, and full E2E validation.
//!
//! Run with:
//! ```bash
//! cargo test --package desktop-app --features robot-app --test robot_real_app -- --ignored
//! ```

#[cfg(all(test, feature = "robot-app"))]
mod real_app_tests {
    use crate::app;
    use compose_testing::robot_app::RobotApp;
    use std::time::Duration;

    /// Test launching the real app with a window
    #[test]
    #[ignore] // Requires display - run with --ignored
    fn test_launch_real_desktop_app() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        // Wait for app to render
        std::thread::sleep(Duration::from_secs(2));

        // Take a screenshot
        let result = robot.screenshot("screenshots/launch_test.png");
        println!("Screenshot result: {:?}", result);

        // Shutdown gracefully
        robot.shutdown().expect("Failed to shutdown");
    }

    /// Test clicking on the counter increment button
    #[test]
    #[ignore]
    fn test_click_increment_button() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        std::thread::sleep(Duration::from_millis(500));

        // Click the increment button (approximate position)
        // In the real app, the increment button is around (150, 560)
        robot.click(150.0, 560.0).expect("Failed to click");

        // Wait for the click to process
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(200));

        // Take screenshot after click
        robot
            .screenshot("screenshots/after_increment.png")
            .expect("Failed to screenshot");

        robot.shutdown().expect("Failed to shutdown");
    }

    /// Test switching tabs
    #[test]
    #[ignore]
    fn test_switch_tabs() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        std::thread::sleep(Duration::from_millis(500));

        // Screenshot initial state (Counter App tab)
        robot
            .screenshot("screenshots/tab_counter.png")
            .expect("Failed to screenshot");

        // Click "Async Runtime" tab (approximate position)
        robot.click(400.0, 50.0).expect("Failed to click");
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(200));

        robot
            .screenshot("screenshots/tab_async.png")
            .expect("Failed to screenshot");

        // Click "Modifiers Showcase" tab
        robot.click(800.0, 50.0).expect("Failed to click");
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(200));

        robot
            .screenshot("screenshots/tab_modifiers.png")
            .expect("Failed to screenshot");

        robot.shutdown().expect("Failed to shutdown");
    }

    /// Test drag interaction
    #[test]
    #[ignore]
    fn test_drag_interaction() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        std::thread::sleep(Duration::from_millis(500));

        // Screenshot before drag
        robot
            .screenshot("screenshots/before_drag.png")
            .expect("Failed to screenshot");

        // Perform a drag gesture
        robot
            .drag(200.0, 300.0, 500.0, 400.0)
            .expect("Failed to drag");

        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(200));

        // Screenshot after drag
        robot
            .screenshot("screenshots/after_drag.png")
            .expect("Failed to screenshot");

        robot.shutdown().expect("Failed to shutdown");
    }

    /// Test window resize
    #[test]
    #[ignore]
    fn test_window_resize() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        std::thread::sleep(Duration::from_millis(500));

        // Screenshot at 800x600
        robot
            .screenshot("screenshots/size_800x600.png")
            .expect("Failed to screenshot");

        // Resize to 1024x768
        robot.resize(1024, 768).expect("Failed to resize");
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(500));

        robot
            .screenshot("screenshots/size_1024x768.png")
            .expect("Failed to screenshot");

        // Resize to mobile-like size
        robot.resize(400, 800).expect("Failed to resize");
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(500));

        robot
            .screenshot("screenshots/size_400x800.png")
            .expect("Failed to screenshot");

        robot.shutdown().expect("Failed to shutdown");
    }

    /// Test complex interaction flow with screenshots
    #[test]
    #[ignore]
    fn test_full_workflow() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        std::thread::sleep(Duration::from_millis(500));

        // Step 1: Initial state
        robot
            .screenshot("screenshots/workflow_01_initial.png")
            .expect("Failed to screenshot");

        // Step 2: Click increment multiple times
        for i in 0..5 {
            robot.click(150.0, 560.0).expect("Failed to click");
            robot.wait_frames(5).expect("Failed to wait");
            std::thread::sleep(Duration::from_millis(100));
        }

        robot
            .screenshot("screenshots/workflow_02_incremented.png")
            .expect("Failed to screenshot");

        // Step 3: Switch to different tab
        robot.click(240.0, 50.0).expect("Failed to click"); // CompositionLocal tab
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(200));

        robot
            .screenshot("screenshots/workflow_03_different_tab.png")
            .expect("Failed to screenshot");

        // Step 4: Go back to counter
        robot.click(70.0, 50.0).expect("Failed to click"); // Counter App tab
        robot.wait_frames(10).expect("Failed to wait");
        std::thread::sleep(Duration::from_millis(200));

        robot
            .screenshot("screenshots/workflow_04_back_to_counter.png")
            .expect("Failed to screenshot");

        robot.shutdown().expect("Failed to shutdown");
    }

    /// Visual regression test - compare screenshots
    #[test]
    #[ignore]
    fn test_visual_regression() {
        let robot = RobotApp::launch(800, 600, || {
            app::combined_app();
        });

        std::thread::sleep(Duration::from_millis(500));

        // Take baseline screenshot
        let (width, height) = robot
            .screenshot("screenshots/baseline.png")
            .expect("Failed to screenshot");

        println!("Baseline screenshot: {}x{}", width, height);

        // In a real test, you would:
        // 1. Compare with a known-good baseline
        // 2. Compute pixel diff
        // 3. Assert diff is below threshold
        //
        // Libraries like image-compare or pixelmatch could be used

        robot.shutdown().expect("Failed to shutdown");
    }
}
