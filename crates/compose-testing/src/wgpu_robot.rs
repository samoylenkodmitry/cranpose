use compose_app_shell::{default_root_key, AppShell};
use compose_render_wgpu::WgpuRenderer;
use std::sync::{mpsc, Arc};

use crate::robot::SceneSnapshot;

/// Offscreen robot harness that drives a full WGPU renderer, mirroring how
/// applications run in production while remaining fully programmatic for
/// testing (pointer moves, presses, releases, and frame captures).
pub struct WgpuRobotApp {
    shell: AppShell<WgpuRenderer>,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    surface_format: wgpu::TextureFormat,
    width: u32,
    height: u32,
}

impl WgpuRobotApp {
    /// Launch a robot-controlled application using the provided viewport size
    /// and renderer fonts.
    pub fn launch_with_fonts(
        width: u32,
        height: u32,
        fonts: &'static [&'static [u8]],
        content: impl FnMut() + 'static,
    ) -> Result<Self, WgpuRobotError> {
        Self::launch_internal(width, height, Some(fonts), content)
    }

    /// Launch a robot-controlled application using the provided viewport size
    /// without bundled fonts. Text rendering will fail unless your UI draws
    /// only shapes.
    pub fn launch(
        width: u32,
        height: u32,
        content: impl FnMut() + 'static,
    ) -> Result<Self, WgpuRobotError> {
        Self::launch_internal(width, height, None, content)
    }

    fn launch_internal(
        width: u32,
        height: u32,
        fonts: Option<&'static [&'static [u8]]>,
        content: impl FnMut() + 'static,
    ) -> Result<Self, WgpuRobotError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or(WgpuRobotError::NoAdapter)?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("robot-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);
        let surface_format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let mut renderer = match fonts {
            Some(fonts) => WgpuRenderer::new_with_fonts(fonts),
            None => WgpuRenderer::new(),
        };
        renderer.init_gpu(device.clone(), queue.clone(), surface_format);
        renderer.set_root_scale(1.0);

        let mut shell = AppShell::new(renderer, default_root_key(), content);
        shell.set_buffer_size(width, height);
        shell.set_viewport(width as f32, height as f32);
        shell.set_frame_waker(|| {});

        let (texture, view) = create_render_target(&device, surface_format, width, height);

        Ok(Self {
            shell,
            device,
            queue,
            texture,
            view,
            surface_format,
            width,
            height,
        })
    }

    /// Resize the viewport and backing texture.
    pub fn set_viewport(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;
        self.shell.set_buffer_size(width, height);
        self.shell.set_viewport(width as f32, height as f32);
        let (texture, view) =
            create_render_target(&self.device, self.surface_format, width, height);
        self.texture = texture;
        self.view = view;
    }

    /// Run a single frame update and render into the offscreen texture.
    pub fn render_frame(&mut self) -> Result<(), WgpuRobotError> {
        self.shell.update();
        self.shell
            .renderer()
            .render(&self.view, self.width, self.height)
            .map_err(|err| WgpuRobotError::Render(format!("{err:?}")))?;
        Ok(())
    }

    /// Drive the application until no redraw is requested or the iteration limit
    /// is reached.
    pub fn pump_until_idle(&mut self, max_iterations: usize) -> Result<(), WgpuRobotError> {
        for _ in 0..max_iterations {
            if !self.shell.needs_redraw() {
                break;
            }
            self.render_frame()?;
        }
        Ok(())
    }

    /// Move the virtual pointer to the provided coordinates, dispatching pointer
    /// move events to any hit targets.
    pub fn move_pointer(&mut self, x: f32, y: f32) -> Result<bool, WgpuRobotError> {
        let moved = self.shell.set_cursor(x, y);
        self.render_frame()?;
        Ok(moved)
    }

    /// Press the virtual pointer at the provided coordinates.
    pub fn press(&mut self, x: f32, y: f32) -> Result<bool, WgpuRobotError> {
        self.shell.set_cursor(x, y);
        let pressed = self.shell.pointer_pressed();
        self.render_frame()?;
        Ok(pressed)
    }

    /// Release the virtual pointer at the provided coordinates.
    pub fn release(&mut self, x: f32, y: f32) -> Result<bool, WgpuRobotError> {
        self.shell.set_cursor(x, y);
        let released = self.shell.pointer_released();
        self.render_frame()?;
        Ok(released)
    }

    /// Convenience helper that presses and releases the pointer at the provided
    /// coordinates.
    pub fn click(&mut self, x: f32, y: f32) -> Result<bool, WgpuRobotError> {
        self.shell.set_cursor(x, y);
        let pressed = self.shell.pointer_pressed();
        let released = self.shell.pointer_released();
        self.render_frame()?;
        Ok(pressed || released)
    }

    /// Snapshot the current render scene for assertions.
    pub fn snapshot(&mut self) -> SceneSnapshot {
        SceneSnapshot::from_wgpu_scene(self.shell.scene())
    }

    /// Capture the currently rendered frame into RGBA bytes suitable for
    /// screenshot comparisons.
    pub fn capture_frame(&mut self) -> Result<FrameCapture, WgpuRobotError> {
        self.render_frame()?;
        let bytes = read_texture_rgba(
            &self.device,
            &self.queue,
            &self.texture,
            self.width,
            self.height,
        )?;
        Ok(FrameCapture {
            width: self.width,
            height: self.height,
            pixels: bytes,
        })
    }

    /// Shut down the robot-controlled application. Dropping the instance will
    /// clean up the underlying shell; this is provided for clarity in tests.
    pub fn close(self) {}
}

/// In-memory screenshot of a rendered frame.
pub struct FrameCapture {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl FrameCapture {
    /// Raw RGBA pixels for the captured frame.
    pub fn rgba(&self) -> &[u8] {
        &self.pixels
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WgpuRobotError {
    #[error("no WGPU adapter available for headless robot run")]
    NoAdapter,
    #[error("failed to create device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("rendering failed: {0}")]
    Render(String),
    #[error("buffer mapping failed: {0}")]
    Map(wgpu::BufferAsyncError),
    #[error("buffer mapping channel dropped before completion")]
    MapChannel,
}

fn create_render_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("robot-framebuffer"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[format],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn read_texture_rgba(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, WgpuRobotError> {
    let bytes_per_pixel = std::mem::size_of::<[u8; 4]>();
    let unpadded_bytes_per_row = width as usize * bytes_per_pixel;
    let padded_bytes_per_row = wgpu::util::align_to(
        unpadded_bytes_per_row,
        wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize,
    );
    let output_buffer_size = (padded_bytes_per_row * height as usize) as wgpu::BufferAddress;

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("robot-readback"),
        size: output_buffer_size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("robot-copy"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);

    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => return Err(WgpuRobotError::Map(err)),
        Err(_) => return Err(WgpuRobotError::MapChannel),
    }

    let data = buffer_slice.get_mapped_range();
    let mut pixels = vec![0u8; width as usize * height as usize * bytes_per_pixel];
    for row in 0..height as usize {
        let src_offset = row * padded_bytes_per_row;
        let dst_offset = row * unpadded_bytes_per_row;
        let src = &data[src_offset..src_offset + unpadded_bytes_per_row];
        pixels[dst_offset..dst_offset + unpadded_bytes_per_row].copy_from_slice(src);
    }

    drop(data);
    buffer.unmap();

    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2); // BGRA → RGBA
    }

    Ok(pixels)
}

#[cfg(test)]
#[path = "tests/wgpu_robot_tests.rs"]
mod tests;
