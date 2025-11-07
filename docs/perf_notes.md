# GPU Text Rendering Performance Notes

## Profiling snapshot
- Source: desktop demo with recursive layout depth 16 and animated "Wave" label.
- Hardware: Vulkan adapter (per user logs) on NVIDIA RTX 2070.
- Sample count: ~72k CPU samples (`cycles:Pu`).
- Dominant cost centers remain in `cosmic_text` shaping and `rustybuzz` layout/planning (~45%+ each).
- GPU render submission and shape batching now account for ~20% of time; staging-buffer churn no longer dominates.

## Interpretation
- Text shaping is still executed every frame for many unique strings (recursive demo labels and animated wave value). Each reflow triggers:
  - `cosmic_text::buffer::set_text` / `shape_until` (glyphon re-shapes the paragraph),
  - `rustybuzz` plan & shape invocations for complex layout features,
  - OpenType table lookups (`ttf_parser::ggg::layout_table::*`).
- The "Wave" animation varies text content every frame (`Wave %.2f`), preventing cache hits and forcing full reshape.
- Layout traversal (`compose_ui::layout::measure_*`) contributes ~12% total, which is expected for a large synthetic tree.

## Recommendations
1. **Reduce per-frame text churn**: For animated numeric strings, consider rendering digits via formatted glyph atlas or limit precision so cache reuse becomes possible (e.g., only update when displayed value changes at 0.05 increments).
2. **Introduce glyph reuse for counters**: Implement substring diffing or per-glyph atlas updates so only changed digits re-shape, avoiding full paragraph shaping.
3. **Precompute recursive demo labels**: Static texts such as "Leaf node #NNNN" can be cached aggressively by supplying stable cache keys rather than formatted strings each frame.
4. **Profile release builds**: The provided trace is likely from a debug build; collecting optimized (`--release`) profiles will lower the absolute cost of shaping and reveal new hot spots.
5. **Parallel shaping exploration**: If shaping remains dominant, experiment with batching multiple `cosmic_text` buffers per frame or exploring parallelization using Rayon.

## Verdict
The profile confirms that recent renderer changes eliminated GPU buffer churn and stabilized glyph caching. Remaining hotspots are inherent to per-frame text re-shaping rather than GPU submission. Performance is acceptable for the demo scenario but further work on reducing text variation or shaping cost will deliver the next gains.
