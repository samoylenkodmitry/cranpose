# compose-render-wgpu

GPU-accelerated renderer backend for rs-compose using WGPU.

## Overview

This crate provides a WGPU-based renderer for the rs-compose UI framework, enabling GPU-accelerated 2D rendering across multiple platforms.

## Cross-Platform Support

WGPU provides excellent cross-platform support:

- **Desktop**: Windows (DX12), macOS (Metal), Linux (Vulkan)
- **Web**: WebGPU (modern browsers)
- **Mobile**: Android (Vulkan), iOS (Metal)

## Current Status

✅ **Implemented:**
- Scene building pipeline (layout tree → render scene)
- Hit testing and pointer event handling
- Text measurement using glyphon
- Basic renderer structure
- Shader infrastructure (WGSL vertex/fragment shaders)
- Support for rectangles, rounded rectangles, and gradients

🚧 **In Progress:**
- Full GPU rendering implementation
- Shape rasterization on GPU
- Text rendering integration

## Architecture

### Components

1. **Scene** (`scene.rs`): Data structures for shapes, text, and hit regions
2. **Pipeline** (`pipeline.rs`): Converts layout tree to render scene
3. **Shaders** (`shaders.rs`): WGSL shaders for 2D primitives
4. **Renderer** (`lib.rs`): Main renderer implementation

### Rendering Features

**Supported Primitives:**
- Rectangles (axis-aligned)
- Rounded rectangles with per-corner radius control
- Solid colors
- Linear gradients
- Radial gradients

## Dependencies

- **wgpu** (0.19): Modern cross-platform graphics API
- **glyphon** (0.5): GPU text rendering
- **bytemuck**: Zero-copy type conversions
- **lru**: Caching for text metrics

## Performance Benefits

Once fully implemented, the GPU renderer will provide:

- **Faster rendering**: Offload rasterization to GPU
- **Better scaling**: Handle complex UIs with many elements
- **Smooth animations**: GPU-accelerated transformations
- **Lower CPU usage**: Free up CPU for application logic

## License

Apache-2.0
