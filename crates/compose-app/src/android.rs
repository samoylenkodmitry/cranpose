//! Android runtime for Compose applications.
//!
//! This module provides the Android event loop implementation with proper
//! lifecycle management, input handling, and rendering coordination.

use crate::launcher::AppSettings;
use compose_app_shell::{default_root_key, AppShell};
use compose_platform_android::AndroidPlatform;
use compose_render_wgpu::WgpuRenderer;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

/// Surface state containing all wgpu resources and the app shell.
struct SurfaceState {
    surface: wgpu::Surface<'static>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    config: wgpu::SurfaceConfiguration,
    app_shell: AppShell<WgpuRenderer>,
}

/// Get display density from Android NDK Configuration.
///
/// Uses the NDK's AConfiguration_getDensity which returns density constants
/// mapped to the standard Android density classes:
/// - mdpi: 1.0 (160 dpi baseline)
/// - hdpi: 1.5 (240 dpi)
/// - xhdpi: 2.0 (320 dpi) - most common modern phones
/// - xxhdpi: 3.0 (480 dpi)
/// - xxxhdpi: 4.0 (640 dpi)
///
/// The factor is calculated as DPI / 160 per Android NDK documentation.
fn get_display_density(app: &android_activity::AndroidApp) -> f32 {
    let config = app.config();
    let density_dpi = config.density(); // Returns Option<u32> with raw DPI value

    // Convert DPI to scale factor (baseline is 160 dpi = 1.0x)
    // e.g., 320 dpi / 160 = 2.0x (xhdpi)
    density_dpi
        .map(|dpi| dpi as f32 / 160.0)
        .unwrap_or(2.0) // Fallback to xhdpi (2.0) if density unavailable
}

/// Renders a single frame. Returns true if out of memory (should exit).
fn render_once(state: &mut SurfaceState) -> bool {
    state.app_shell.update();

    match state.surface.get_current_texture() {
        Ok(frame) => {
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            let (width, height) = state.app_shell.buffer_size();

            if let Err(e) = state.app_shell.renderer().render(&view, width, height) {
                log::error!("Render error: {:?}", e);
            }

            frame.present();
            false
        }
        Err(wgpu::SurfaceError::Lost) | Err(wgpu::SurfaceError::Outdated) => {
            // Reconfigure surface using current size and config
            let (width, height) = state.app_shell.buffer_size();
            state.config.width = width;
            state.config.height = height;
            state.surface.configure(&state.device, &state.config);
            false
        }
        Err(wgpu::SurfaceError::OutOfMemory) => {
            log::error!("Out of memory; exiting");
            true
        }
        Err(e) => {
            log::debug!("Surface error: {:?}", e);
            false
        }
    }
}

/// Runs an Android Compose application with wgpu rendering.
///
/// Called by `AppLauncher::run_android()`. This is the framework-level
/// entrypoint that manages the Android lifecycle and event loop.
///
/// **Note:** Applications should use `AppLauncher` instead of calling this directly.
pub fn run(
    app: android_activity::AndroidApp,
    _settings: AppSettings,
    content: impl FnMut() + 'static,
) {
    use android_activity::{input::MotionAction, InputStatus, MainEvent, PollEvent};

    // Wrap content in Option so we can move it out when creating AppShell
    let mut content = Some(content);

    // Initialize logging
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("ComposeRS")
            .with_filter(
                android_logger::FilterBuilder::new()
                    .filter_level(log::LevelFilter::Info)
                    .filter_module("wgpu_core", log::LevelFilter::Warn)
                    .filter_module("wgpu_hal", log::LevelFilter::Warn)
                    .filter_module("naga", log::LevelFilter::Warn)
                    .build(),
            ),
    );

    log::info!("Starting Compose Android Application");

    // Frame wake flag for event-driven rendering
    let need_frame = Arc::new(AtomicBool::new(false));

    // Initialize wgpu instance with GL and Vulkan backends
    // GL works better on emulators, but Vulkan is preferred on real devices
    let backends = wgpu::Backends::GL | wgpu::Backends::VULKAN;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });

    // Platform abstraction for density/pointer conversion
    let mut android_platform = AndroidPlatform::new();

    // Surface state (initialized when window is ready)
    let mut surface_state: Option<SurfaceState> = None;

    // Main event loop
    loop {
        // Dynamic poll duration:
        // - None when no window (event-driven sleep)
        // - ZERO when animating (tight loop)
        // - None when idle (event-driven)
        let poll_duration = if surface_state.is_none() {
            None // No window, sleep until next event
        } else if let Some(state) = &surface_state {
            if state.app_shell.has_active_animations() {
                Some(std::time::Duration::ZERO) // Tight loop for animations
            } else {
                None // Idle, sleep until next event
            }
        } else {
            None
        };

        app.poll_events(poll_duration, |event| {
            match event {
                PollEvent::Main(main_event) => match main_event {
                    MainEvent::InitWindow { .. } => {
                        log::info!("Window initialized, setting up rendering");

                        if let Some(native_window) = app.native_window() {
                            // Get actual window dimensions
                            let width = native_window.width() as u32;
                            let height = native_window.height() as u32;

                            // Create surface from the Android window using platform helper
                            let surface = unsafe {
                                compose_platform_android::create_wgpu_surface(&instance, &native_window)
                                    .expect("Failed to create surface")
                            };

                            // Request adapter
                            let adapter = pollster::block_on(instance.request_adapter(
                                &wgpu::RequestAdapterOptions {
                                    power_preference: wgpu::PowerPreference::HighPerformance,
                                    compatible_surface: Some(&surface),
                                    force_fallback_adapter: false,
                                },
                            ))
                            .expect("Failed to find suitable adapter");

                            let adapter_info = adapter.get_info();
                            log::info!("Found adapter: {:?}", adapter_info.backend);

                            // Request device and queue
                            let (device, queue) = pollster::block_on(adapter.request_device(
                                &wgpu::DeviceDescriptor {
                                    label: Some("Android Device"),
                                    required_features: wgpu::Features::empty(),
                                    required_limits: wgpu::Limits::default(),
                                },
                                None,
                            ))
                            .expect("Failed to create device");

                            let device = Arc::new(device);
                            let queue = Arc::new(queue);

                            // Get surface capabilities and format
                            let surface_caps = surface.get_capabilities(&adapter);
                            let surface_format = surface_caps
                                .formats
                                .iter()
                                .copied()
                                .find(|f| f.is_srgb())
                                .unwrap_or(surface_caps.formats[0]);

                            // Configure surface
                            let surface_config = wgpu::SurfaceConfiguration {
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                format: surface_format,
                                width,
                                height,
                                present_mode: wgpu::PresentMode::Fifo,
                                alpha_mode: surface_caps.alpha_modes[0],
                                view_formats: vec![],
                                desired_maximum_frame_latency: 2,
                            };

                            surface.configure(&device, &surface_config);

                            // Get display density and update platform
                            let density = get_display_density(&app);
                            android_platform.set_scale_factor(density as f64);
                            log::info!("Display density: {:.2}x", density);

                            // Load bundled fonts
                            let font_light = include_bytes!("../../../assets/Roboto-Light.ttf");
                            let font_regular = include_bytes!("../../../assets/Roboto-Regular.ttf");

                            // Create renderer with fonts
                            let mut renderer = WgpuRenderer::new_with_fonts(&[font_light, font_regular]);
                            renderer.init_gpu(device.clone(), queue.clone(), surface_format);
                            renderer.set_root_scale(density);

                            // Create app shell with content (take from Option)
                            let mut app_shell = AppShell::new(
                                renderer,
                                default_root_key(),
                                content.take().expect("content already used"),
                            );

                            // Wire frame waker for event-driven rendering
                            {
                                let need_frame = need_frame.clone();
                                app_shell.set_frame_waker(move || {
                                    need_frame.store(true, Ordering::Relaxed);
                                });
                            }

                            // Set buffer_size to physical pixels
                            app_shell.set_buffer_size(width, height);

                            // Set viewport to logical dp
                            let width_dp = width as f32 / density;
                            let height_dp = height as f32 / density;
                            app_shell.set_viewport(width_dp, height_dp);
                            log::info!(
                                "Set viewport to {:.1}x{:.1} dp ({}x{} px at {:.2}x density)",
                                width_dp,
                                height_dp,
                                width,
                                height,
                                density
                            );

                            surface_state = Some(SurfaceState {
                                surface,
                                device,
                                queue,
                                config: surface_config,
                                app_shell,
                            });

                            log::info!("Rendering initialized successfully");
                        }
                    }
                    MainEvent::TerminateWindow { .. } => {
                        log::info!("Window terminated");
                        surface_state = None;
                    }
                    MainEvent::WindowResized { .. } => {
                        if let Some(native_window) = app.native_window() {
                            let width = native_window.width() as u32;
                            let height = native_window.height() as u32;

                            let density = get_display_density(&app);
                            android_platform.set_scale_factor(density as f64);
                            log::info!(
                                "Window resized to {}x{} at {:.2}x density",
                                width,
                                height,
                                density
                            );

                            if let Some(state) = &mut surface_state {
                                if width > 0 && height > 0 {
                                    state.config.width = width;
                                    state.config.height = height;
                                    state.surface.configure(&state.device, &state.config);

                                    // Set buffer_size to physical pixels
                                    state.app_shell.set_buffer_size(width, height);

                                    // Set viewport to logical dp (marks dirty internally)
                                    let width_dp = width as f32 / density;
                                    let height_dp = height as f32 / density;
                                    state.app_shell.set_viewport(width_dp, height_dp);

                                    // Update renderer scale
                                    state.app_shell.renderer().set_root_scale(density);
                                }
                            }
                        }
                    }
                    MainEvent::RedrawNeeded { .. } => {
                        if let Some(state) = &mut surface_state {
                            state.app_shell.mark_dirty();
                        }
                    }
                    _ => {}
                },
                // Handle input events to prevent ANR
                _ => {
                    if let Ok(mut iter) = app.input_events_iter() {
                        loop {
                            if !iter.next(|event| {
                                let handled = match event {
                                    android_activity::input::InputEvent::MotionEvent(
                                        motion_event,
                                    ) => {
                                        // Get pointer position in physical pixels and convert to logical dp
                                        let pointer = motion_event.pointer_at_index(0);
                                        let x_px = pointer.x() as f64;
                                        let y_px = pointer.y() as f64;
                                        let logical = android_platform.pointer_position(x_px, y_px);

                                        match motion_event.action() {
                                            MotionAction::Down | MotionAction::PointerDown => {
                                                if let Some(state) = &mut surface_state {
                                                    state.app_shell.set_cursor(logical.x, logical.y);
                                                    state.app_shell.pointer_pressed();
                                                }
                                            }
                                            MotionAction::Up | MotionAction::PointerUp => {
                                                if let Some(state) = &mut surface_state {
                                                    state.app_shell.set_cursor(logical.x, logical.y);
                                                    state.app_shell.pointer_released();
                                                }
                                            }
                                            MotionAction::Move => {
                                                if let Some(state) = &mut surface_state {
                                                    state.app_shell.set_cursor(logical.x, logical.y);
                                                }
                                            }
                                            _ => {}
                                        }
                                        true
                                    }
                                    _ => false,
                                };

                                if handled {
                                    InputStatus::Handled
                                } else {
                                    InputStatus::Unhandled
                                }
                            }) {
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Check if app side requested a frame (animations, state changes)
        if need_frame.swap(false, Ordering::Relaxed) {
            if let Some(state) = &mut surface_state {
                state.app_shell.mark_dirty();
            }
        }

        // Render outside event callback if needed
        if let Some(state) = &mut surface_state {
            if state.app_shell.needs_redraw() {
                if render_once(state) {
                    break; // Out of memory, exit
                }
            }
        }
    }
}
