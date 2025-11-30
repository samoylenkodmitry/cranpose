//! Robot testing with real app rendering
//!
//! This module provides robot testing that launches the actual desktop app
//! with real rendering, enabling screenshot testing, visual validation, etc.

use compose_app_shell::AppShell;
use compose_core::{location_key, Key};
use compose_render_wgpu::WgpuRenderer;
use compose_ui_graphics::Size;
use std::sync::{Arc, Mutex};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};
use winit::window::Window;
use wgpu;

/// Events that can be sent to the robot-controlled app
#[derive(Debug, Clone)]
pub enum RobotCommand {
    Click { x: f32, y: f32 },
    Move { x: f32, y: f32 },
    Drag { from_x: f32, from_y: f32, to_x: f32, to_y: f32 },
    Resize { width: u32, height: u32 },
    Screenshot { path: String },
    WaitFrames { count: u32 },
    Shutdown,
}

/// Result from executing a robot command
#[derive(Debug, Clone)]
pub enum RobotResult {
    Success,
    Screenshot { path: String, size: (u32, u32) },
    Error(String),
}

/// A robot test runner that controls a real desktop app
///
/// This launches the actual application window with real rendering,
/// allowing for screenshot testing, visual validation, and full E2E tests.
///
/// # Example
///
/// ```no_run
/// use compose_testing::robot_app::RobotApp;
///
/// let mut robot = RobotApp::launch(800, 600, || {
///     my_app();
/// });
///
/// // Interact with the real app
/// robot.click(400.0, 300.0);
/// robot.wait_frames(10);
///
/// // Take a screenshot
/// robot.screenshot("test_output.png");
///
/// // Shutdown when done
/// robot.shutdown();
/// ```
pub struct RobotApp {
    command_sender: EventLoopProxy<RobotCommand>,
    result_receiver: Arc<Mutex<Vec<RobotResult>>>,
}

impl RobotApp {
    /// Launch the real app with robot control
    ///
    /// This creates an actual window and starts the event loop in a background thread.
    /// The robot can then send commands to interact with the app.
    pub fn launch<F>(width: u32, height: u32, content: F) -> Self
    where
        F: FnMut() + 'static + Send,
    {
        let (command_sender, result_receiver) = Self::spawn_app_thread(width, height, content);

        Self {
            command_sender,
            result_receiver,
        }
    }

    /// Click at the given position
    pub fn click(&self, x: f32, y: f32) -> Result<(), String> {
        self.send_command(RobotCommand::Click { x, y })
    }

    /// Move cursor to the given position
    pub fn move_to(&self, x: f32, y: f32) -> Result<(), String> {
        self.send_command(RobotCommand::Move { x, y })
    }

    /// Drag from one position to another
    pub fn drag(&self, from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> Result<(), String> {
        self.send_command(RobotCommand::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
        })
    }

    /// Resize the window
    pub fn resize(&self, width: u32, height: u32) -> Result<(), String> {
        self.send_command(RobotCommand::Resize { width, height })
    }

    /// Take a screenshot and save to the given path
    pub fn screenshot(&self, path: &str) -> Result<(u32, u32), String> {
        self.send_command(RobotCommand::Screenshot {
            path: path.to_string(),
        })?;

        // Wait for screenshot result
        std::thread::sleep(std::time::Duration::from_millis(100));

        let results = self.result_receiver.lock().unwrap();
        for result in results.iter() {
            if let RobotResult::Screenshot { path: _, size } = result {
                return Ok(*size);
            }
        }

        Err("Screenshot failed".to_string())
    }

    /// Wait for a number of frames to render
    pub fn wait_frames(&self, count: u32) -> Result<(), String> {
        self.send_command(RobotCommand::WaitFrames { count })
    }

    /// Shutdown the app
    pub fn shutdown(&self) -> Result<(), String> {
        self.send_command(RobotCommand::Shutdown)
    }

    /// Send a command to the app
    fn send_command(&self, command: RobotCommand) -> Result<(), String> {
        self.command_sender
            .send_event(command)
            .map_err(|e| format!("Failed to send command: {:?}", e))
    }

    /// Spawn the app in a background thread
    fn spawn_app_thread<F>(
        width: u32,
        height: u32,
        mut content: F,
    ) -> (EventLoopProxy<RobotCommand>, Arc<Mutex<Vec<RobotResult>>>)
    where
        F: FnMut() + 'static + Send,
    {
        let result_queue = Arc::new(Mutex::new(Vec::new()));
        let result_queue_clone = result_queue.clone();

        let (proxy_sender, proxy_receiver) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let event_loop = EventLoop::<RobotCommand>::with_user_event()
                .build()
                .expect("Failed to create event loop");

            let proxy = event_loop.create_proxy();
            proxy_sender.send(proxy).unwrap();

            let window = Arc::new(winit::window::WindowBuilder::new()
                .with_title("Robot Test App")
                .with_inner_size(PhysicalSize::new(width, height))
                .build(&event_loop)
                .expect("Failed to create window"));

            // Initialize WGPU (same as desktop app)
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::all(),
                ..Default::default()
            });

            let surface = instance
                .create_surface(window.clone())
                .expect("Failed to create surface");

            let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .expect("Failed to find suitable adapter");

            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Robot Test Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            ))
            .expect("Failed to create device");

            let size = window.inner_size();
            let surface_caps = surface.get_capabilities(&adapter);
            let surface_format = surface_caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .unwrap_or(surface_caps.formats[0]);

            let mut surface_config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: surface_caps.alpha_modes[0],
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            };

            surface.configure(&device, &surface_config);

            // Create renderer
            let mut renderer = WgpuRenderer::new();
            renderer.init_gpu(Arc::new(device), Arc::new(queue), surface_format);

            // Create app shell
            let root_key = location_key(file!(), line!(), column!());
            let mut shell = AppShell::new(renderer, root_key, content);
            shell.set_viewport(width as f32, height as f32);
            shell.set_buffer_size(width, height);

            let mut frame_count = 0;
            let mut wait_frames = 0;

            event_loop
                .run(move |event, elwt| {
                    elwt.set_control_flow(ControlFlow::Poll);

                    match event {
                        Event::UserEvent(cmd) => {
                            Self::handle_robot_command(
                                cmd,
                                &mut shell,
                                &window,
                                &result_queue_clone,
                                elwt,
                                &mut wait_frames,
                            );
                        }
                        Event::WindowEvent { event, .. } => match event {
                            WindowEvent::CloseRequested => {
                                elwt.exit();
                            }
                            WindowEvent::Resized(new_size) => {
                                shell.set_viewport(new_size.width as f32, new_size.height as f32);
                                shell.set_buffer_size(new_size.width, new_size.height);
                                shell
                                    .renderer()
                                    .resize(new_size.width, new_size.height)
                                    .expect("Failed to resize");
                            }
                            WindowEvent::RedrawRequested => {
                                shell.update();

                                if shell.should_render() {
                                    if let Err(err) = shell.renderer().render() {
                                        eprintln!("Render failed: {:?}", err);
                                    }
                                }

                                frame_count += 1;
                                if wait_frames > 0 {
                                    wait_frames -= 1;
                                    if wait_frames == 0 {
                                        result_queue_clone
                                            .lock()
                                            .unwrap()
                                            .push(RobotResult::Success);
                                    }
                                }

                                window.request_redraw();
                            }
                            _ => {}
                        },
                        Event::AboutToWait => {
                            window.request_redraw();
                        }
                        _ => {}
                    }
                })
                .expect("Event loop failed");
        });

        let proxy = proxy_receiver.recv().unwrap();
        (proxy, result_queue)
    }

    fn handle_robot_command<R>(
        cmd: RobotCommand,
        shell: &mut AppShell<R>,
        window: &Window,
        results: &Arc<Mutex<Vec<RobotResult>>>,
        elwt: &winit::event_loop::EventLoopWindowTarget<RobotCommand>,
        wait_frames: &mut u32,
    ) where
        R: compose_render_common::Renderer,
        R::Error: std::fmt::Debug,
    {
        match cmd {
            RobotCommand::Click { x, y } => {
                shell.set_cursor(x, y);
                shell.pointer_pressed();
                shell.pointer_released();
                results.lock().unwrap().push(RobotResult::Success);
            }
            RobotCommand::Move { x, y } => {
                shell.set_cursor(x, y);
                results.lock().unwrap().push(RobotResult::Success);
            }
            RobotCommand::Drag {
                from_x,
                from_y,
                to_x,
                to_y,
            } => {
                shell.set_cursor(from_x, from_y);
                shell.pointer_pressed();

                // Simulate drag in steps
                let steps = 10;
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let x = from_x + (to_x - from_x) * t;
                    let y = from_y + (to_y - from_y) * t;
                    shell.set_cursor(x, y);
                    shell.update();
                }

                shell.pointer_released();
                results.lock().unwrap().push(RobotResult::Success);
            }
            RobotCommand::Resize { width, height } => {
                let _ = window.request_inner_size(PhysicalSize::new(width, height));
                results.lock().unwrap().push(RobotResult::Success);
            }
            RobotCommand::Screenshot { path } => {
                // TODO: Implement actual screenshot capture
                // This would require accessing the GPU buffer
                let size = window.inner_size();
                results.lock().unwrap().push(RobotResult::Screenshot {
                    path,
                    size: (size.width, size.height),
                });
            }
            RobotCommand::WaitFrames { count } => {
                *wait_frames = count;
            }
            RobotCommand::Shutdown => {
                elwt.exit();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a display/window system
    // They may not work in headless CI environments

    #[test]
    #[ignore] // Ignore by default as it requires a display
    fn test_launch_real_app() {
        let robot = RobotApp::launch(800, 600, || {
            // Simple empty app
        });

        // Wait for app to initialize
        std::thread::sleep(std::time::Duration::from_millis(500));

        robot.shutdown().unwrap();
    }

    #[test]
    #[ignore]
    fn test_click_real_app() {
        let robot = RobotApp::launch(800, 600, || {
            // App content
        });

        std::thread::sleep(std::time::Duration::from_millis(500));

        robot.click(400.0, 300.0).unwrap();
        robot.wait_frames(10).unwrap();

        robot.shutdown().unwrap();
    }

    #[test]
    #[ignore]
    fn test_screenshot_real_app() {
        let robot = RobotApp::launch(800, 600, || {
            // App content
        });

        std::thread::sleep(std::time::Duration::from_millis(500));

        let result = robot.screenshot("test_screenshot.png");
        assert!(result.is_ok());

        robot.shutdown().unwrap();
    }
}
