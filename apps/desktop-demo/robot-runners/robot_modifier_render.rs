//! Robot test to reproduce modifier showcase rendering issue
//! Tests that clicking "Positioned Boxes" in modifier tab shows content

use compose_app::AppLauncher;
use compose_testing::{find_button, find_in_semantics, find_text};
use desktop_app::app::combined_app;
use desktop_app::fonts::DEMO_FONTS;
use std::time::Duration;

fn main() {
    AppLauncher::new()
        .with_title("Robot: Modifier Render Test")
        .with_size(1200, 900)
        .with_fonts(&DEMO_FONTS)
        .with_headless(true)
        .with_fps_counter(true)
        .with_test_driver(|robot| {
            std::thread::sleep(Duration::from_millis(500));
            robot.wait_for_idle().expect("wait for idle");
            println!("✓ App launched");

            // Click on Modifiers Showcase tab (note: with 's')
            if let Some((x, y, w, h)) = find_in_semantics(&robot, |elem| find_button(elem, "Modifiers Showcase")) {
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                robot.click(cx, cy).expect("click modifiers tab");
                robot.wait_for_idle().expect("wait after tab click");
                println!("✓ Clicked Modifiers Showcase tab at ({:.1}, {:.1})", cx, cy);
            } else {
                println!("✗ FAIL: Could not find Modifiers Showcase tab");
                robot.exit().expect("exit");
                return;
            }

            std::thread::sleep(Duration::from_millis(200));

            // Look for Positioned Boxes button
            if let Some((x, y, w, h)) = find_in_semantics(&robot, |elem| find_button(elem, "Positioned Boxes")) {
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                println!("  Found 'Positioned Boxes' at ({:.1}, {:.1})", cx, cy);
                
                robot.click(cx, cy).expect("click Positioned Boxes");
                robot.wait_for_idle().expect("wait after click");
                println!("✓ Clicked Positioned Boxes");
            } else {
                println!("✗ FAIL: Could not find Positioned Boxes button");
                robot.exit().expect("exit");
                return;
            }

            std::thread::sleep(Duration::from_millis(500));
            robot.wait_for_idle().expect("wait for content");

            // Check for expected content - look for "Layer" text which should appear
            let has_layer = find_in_semantics(&robot, |elem| find_text(elem, "Layer"));
            let has_box = find_in_semantics(&robot, |elem| find_text(elem, "Box"));
            
            if has_layer.is_some() || has_box.is_some() {
                println!("  ✓ PASS: Content found after clicking Positioned Boxes");
                println!("✓ ALL TESTS PASSED");
            } else {
                // This is the regression - content should be visible after clicking
                println!("  ✗ FAIL: No content visible after clicking Positioned Boxes!");
                println!("         Expected to find 'Layer' or 'Box' text");
                println!("         This is the recomposition regression!");
            }

            robot.exit().expect("exit");
        })
        .run(combined_app);
}
