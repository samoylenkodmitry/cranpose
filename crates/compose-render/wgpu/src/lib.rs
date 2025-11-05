//! WGPU renderer backend for GPU-accelerated 2D rendering.
//!
//! This is a work-in-progress GPU renderer that aims to accelerate
//! the 2D rendering using WGPU for cross-platform GPU support.

mod pipeline;
mod scene;
mod shaders;

pub use scene::{ClickAction, DrawShape, HitRegion, Scene, TextDraw};

use compose_render_common::{Renderer, RenderScene};
use compose_ui::{set_text_measurer, LayoutTree, TextMeasurer};
use compose_ui_graphics::Size;
use glyphon::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping};
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum WgpuRendererError {
    Layout(String),
    Wgpu(String),
}

/// WGPU-based renderer for GPU-accelerated 2D rendering.
///
/// Currently under development - scene building works but GPU rendering
/// is not yet fully implemented.
pub struct WgpuRenderer {
    scene: Scene,
    _font_system: Arc<Mutex<FontSystem>>,
}

impl WgpuRenderer {
    pub fn new() -> Self {
        let font_system = Arc::new(Mutex::new(FontSystem::new()));
        let text_measurer = WgpuTextMeasurer::new(font_system.clone());
        set_text_measurer(text_measurer.clone());

        Self {
            scene: Scene::new(),
            _font_system: font_system,
        }
    }
}

impl Default for WgpuRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer for WgpuRenderer {
    type Scene = Scene;
    type Error = WgpuRendererError;

    fn scene(&self) -> &Self::Scene {
        &self.scene
    }

    fn scene_mut(&mut self) -> &mut Self::Scene {
        &mut self.scene
    }

    fn rebuild_scene(
        &mut self,
        layout_tree: &LayoutTree,
        _viewport: Size,
    ) -> Result<(), Self::Error> {
        self.scene.clear();
        pipeline::render_layout_tree(layout_tree.root(), &mut self.scene);
        Ok(())
    }
}

// Text measurer implementation for WGPU
#[derive(Clone)]
struct WgpuTextMeasurer {
    font_system: Arc<Mutex<FontSystem>>,
    cache: Arc<Mutex<LruCache<(String, i32), Size>>>,
}

impl WgpuTextMeasurer {
    fn new(font_system: Arc<Mutex<FontSystem>>) -> Self {
        Self {
            font_system,
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(64).unwrap()))),
        }
    }
}

impl TextMeasurer for WgpuTextMeasurer {
    fn measure(&self, text: &str) -> compose_ui::TextMetrics {
        let font_size = 14.0; // Default font size
        let key = (text.to_string(), (font_size * 100.0) as i32);

        {
            let mut cache = self.cache.lock().unwrap();
            if let Some(size) = cache.get(&key) {
                return compose_ui::TextMetrics {
                    width: size.width,
                    height: size.height,
                };
            }
        }

        let mut font_system = self.font_system.lock().unwrap();
        let mut buffer = Buffer::new(&mut font_system, Metrics::new(font_size, font_size * 1.4));
        buffer.set_size(&mut font_system, f32::MAX, f32::MAX);
        buffer.set_text(
            &mut font_system,
            text,
            Attrs::new().family(Family::SansSerif),
            Shaping::Advanced,
        );
        buffer.shape_until_scroll(&mut font_system);

        let mut max_width = 0.0f32;
        let mut total_height = 0.0f32;

        for line in buffer.lines.iter() {
            let layout_runs = buffer.layout_runs();
            for run in layout_runs {
                max_width = max_width.max(run.line_w);
                break; // Just get the first run's width
            }
            total_height += font_size * 1.4;
        }

        let size = Size {
            width: max_width,
            height: total_height,
        };

        let mut cache = self.cache.lock().unwrap();
        cache.put(key, size);

        compose_ui::TextMetrics {
            width: size.width,
            height: size.height,
        }
    }
}
