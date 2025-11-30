#![allow(clippy::type_complexity)]

mod hit_path_tracker;

use std::fmt::Debug;
// Use instant crate for cross-platform time support (native + WASM)
use instant::Instant;

use compose_core::{location_key, Applier, Composition, Key, MemoryApplier, NodeError};
use compose_foundation::{PointerButton, PointerButtons, PointerEvent, PointerEventKind};
use compose_render_common::{HitTestTarget, RenderScene, Renderer};
use compose_runtime_std::StdRuntime;
use compose_ui::{
    has_pending_focus_invalidations, has_pending_pointer_repasses, log_layout_tree,
    log_render_scene, log_screen_summary, peek_focus_invalidation, peek_pointer_invalidation,
    peek_render_invalidation, peek_layout_invalidation, process_focus_invalidations, process_pointer_repasses,
    request_render_invalidation, take_focus_invalidation, take_pointer_invalidation,
    take_render_invalidation, take_layout_invalidation, HeadlessRenderer, LayoutNode, LayoutTree, SemanticsTree,
};
use compose_ui_graphics::{Point, Size};
use hit_path_tracker::{HitPathTracker, PointerId};

pub struct AppShell<R>
where
    R: Renderer,
{
    runtime: StdRuntime,
    composition: Composition<MemoryApplier>,
    renderer: R,
    cursor: (f32, f32),
    viewport: (f32, f32),
    buffer_size: (u32, u32),
    start_time: Instant,
    layout_tree: Option<LayoutTree>,
    semantics_tree: Option<SemanticsTree>,
    layout_dirty: bool,
    scene_dirty: bool,
    is_dirty: bool,
    /// Tracks which mouse buttons are currently pressed
    buttons_pressed: PointerButtons,
    /// Tracks which nodes were hit on PointerDown (by stable NodeId).
    /// 
    /// This follows Jetpack Compose's HitPathTracker pattern:
    /// - On Down: cache NodeIds, not geometry
    /// - On Move/Up/Cancel: resolve fresh HitTargets from current scene
    /// - Handler closures are preserved (same Rc), so internal state survives
    hit_path_tracker: HitPathTracker,

}

impl<R> AppShell<R>
where
    R: Renderer,
    R::Error: Debug,
{
    pub fn new(mut renderer: R, root_key: Key, content: impl FnMut() + 'static) -> Self {
        let runtime = StdRuntime::new();
        let mut composition = Composition::with_runtime(MemoryApplier::new(), runtime.runtime());
        let build = content;
        if let Err(err) = composition.render(root_key, build) {
            log::error!("initial render failed: {err}");
        }
        renderer.scene_mut().clear();
        let mut shell = Self {
            runtime,
            composition,
            renderer,
            cursor: (0.0, 0.0),
            viewport: (800.0, 600.0),
            buffer_size: (800, 600),
            start_time: Instant::now(),
            layout_tree: None,
            semantics_tree: None,
            layout_dirty: true,
            scene_dirty: true,
            is_dirty: true,
            buttons_pressed: PointerButtons::NONE,
            hit_path_tracker: HitPathTracker::new(),

        };
        shell.process_frame();
        shell
    }

    pub fn set_viewport(&mut self, width: f32, height: f32) {
        self.viewport = (width, height);
        self.layout_dirty = true;
        self.mark_dirty();
        self.process_frame();
    }

    pub fn set_buffer_size(&mut self, width: u32, height: u32) {
        self.buffer_size = (width, height);
    }

    pub fn buffer_size(&self) -> (u32, u32) {
        self.buffer_size
    }

    pub fn scene(&self) -> &R::Scene {
        self.renderer.scene()
    }

    pub fn renderer(&mut self) -> &mut R {
        &mut self.renderer
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_frame_waker(&mut self, waker: impl Fn() + Send + Sync + 'static) {
        self.runtime.set_frame_waker(waker);
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_frame_waker(&mut self, waker: impl Fn() + Send + 'static) {
        self.runtime.set_frame_waker(waker);
    }

    pub fn clear_frame_waker(&mut self) {
        self.runtime.clear_frame_waker();
    }

    pub fn should_render(&self) -> bool {
        if self.layout_dirty
            || self.scene_dirty
            || peek_render_invalidation()
            || peek_pointer_invalidation()
            || peek_focus_invalidation()
            || peek_layout_invalidation()
        {
            return true;
        }
        self.runtime.take_frame_request() || self.composition.should_render()
    }

    /// Returns true if the shell needs to redraw (dirty flag or active animations).
    pub fn needs_redraw(&self) -> bool {
        self.is_dirty || self.has_active_animations()
    }

    /// Marks the shell as dirty, indicating a redraw is needed.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Returns true if there are active animations or pending recompositions.
    pub fn has_active_animations(&self) -> bool {
        self.runtime.take_frame_request() || self.composition.should_render()
    }

    /// Resolves cached NodeIds to fresh HitTargets from the current scene.
    ///
    /// This is the key to avoiding stale geometry during scroll/layout changes:
    /// - We cache NodeIds on PointerDown (stable identity)
    /// - On Move/Up/Cancel, we call find_target() to get fresh geometry
    /// - Handler closures are preserved (same Rc), so gesture state survives
    fn resolve_hit_path(&self, pointer: PointerId) -> Vec<<<R as Renderer>::Scene as RenderScene>::HitTarget> {
        let Some(node_ids) = self.hit_path_tracker.get_path(pointer) else {
            return Vec::new();
        };
        
        let scene = self.renderer.scene();
        node_ids
            .iter()
            .filter_map(|&id| scene.find_target(id))
            .collect()
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let frame_time = now
            .checked_duration_since(self.start_time)
            .unwrap_or_default()
            .as_nanos() as u64;
        self.runtime.drain_frame_callbacks(frame_time);
        self.runtime.runtime_handle().drain_ui();
        if self.composition.should_render() {
            match self.composition.process_invalid_scopes() {
                Ok(changed) => {
                    if changed {
                        self.layout_dirty = true;
                        // Request render invalidation so the scene gets rebuilt
                        request_render_invalidation();
                    }
                }
                Err(NodeError::Missing { id }) => {
                    // Node was removed (likely due to conditional render or tab switch)
                    // This is expected when scopes try to recompose after their nodes are gone
                    log::debug!("Recomposition skipped: node {} no longer exists", id);
                    self.layout_dirty = true;
                    request_render_invalidation();
                }
                Err(err) => {
                    log::error!("recomposition failed: {err}");
                    self.layout_dirty = true;
                    request_render_invalidation();
                }
            }
        }
        self.process_frame();
        // Clear dirty flag after update (frame has been processed)
        self.is_dirty = false;
    }

    pub fn set_cursor(&mut self, x: f32, y: f32) -> bool {
        self.cursor = (x, y);
        
        // During a gesture (button pressed), ONLY dispatch to the tracked hit path.
        // Never fall back to hover hit-testing while buttons are down.
        // This maintains the invariant: the path that receives Down must receive Move and Up/Cancel.
        if self.buttons_pressed != PointerButtons::NONE {
            if self.hit_path_tracker.has_path(PointerId::PRIMARY) {
                // Resolve fresh targets from current scene (not cached geometry!)
                let targets = self.resolve_hit_path(PointerId::PRIMARY);
                
                if !targets.is_empty() {
                    let event = PointerEvent::new(
                        PointerEventKind::Move,
                        Point { x, y },
                        Point { x, y },
                    ).with_buttons(self.buttons_pressed);
                    
                    for hit in targets {
                        hit.dispatch(event.clone());
                        if event.is_consumed() {
                            break;
                        }
                    }
                    self.mark_dirty();
                    return true;
                }
                
                // Gesture exists but we can't resolve any nodes (removed / no hit region).
                // Do NOT switch to hover mode while buttons are pressed.
                return false;
            }
            
            // Button is down but we have no recorded path inside this app
            // (e.g. drag started outside). Do not dispatch anything.
            return false;
        }
        
        // No gesture in progress: regular hover move using hit-test.
        let hits = self.renderer.scene().hit_test(x, y);
        if !hits.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Move,
                Point { x, y },
                Point { x, y },
            ).with_buttons(self.buttons_pressed); // usually NONE here
            for hit in hits {
                hit.dispatch(event.clone());
                if event.is_consumed() {
                    break;
                }
            }
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn pointer_pressed(&mut self) -> bool {
        // Track button state
        self.buttons_pressed.insert(PointerButton::Primary);
        
        // Hit-test against the current (last rendered) scene.
        // Even if the app is dirty, this scene is what the user actually saw and clicked.
        // Frame N is rendered → user sees frame N and taps → we hit-test frame N's geometry.
        // The pointer event may mark dirty → next frame runs update() → renders N+1.
        
        // Perform hit test and cache the NodeIds (not geometry!)
        // The key insight from Jetpack Compose: cache identity, resolve fresh geometry per dispatch
        let hits = self.renderer.scene().hit_test(self.cursor.0, self.cursor.1);
        
        // Cache NodeIds for this pointer
        let node_ids: Vec<_> = hits.iter().map(|h| h.node_id()).collect();
        self.hit_path_tracker.add_hit_path(PointerId::PRIMARY, node_ids);
        
        if !hits.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Down,
                Point { x: self.cursor.0, y: self.cursor.1 },
                Point { x: self.cursor.0, y: self.cursor.1 },
            ).with_buttons(self.buttons_pressed);
            
            // Dispatch to fresh hits (geometry is already current for Down event)
            for hit in hits {
                hit.dispatch(event.clone());
                if event.is_consumed() {
                    break;
                }
            }
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn pointer_released(&mut self) -> bool {
        // UP events report buttons as "currently pressed" (after release),
        // matching typical platform semantics where primary is already gone.
        self.buttons_pressed.remove(PointerButton::Primary);
        let corrected_buttons = self.buttons_pressed;
        
        // Resolve FRESH targets from cached NodeIds
        let targets = self.resolve_hit_path(PointerId::PRIMARY);
        
        // Always remove the path, even if targets is empty (node may have been removed)
        self.hit_path_tracker.remove_path(PointerId::PRIMARY);
        
        if !targets.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Up,
                Point { x: self.cursor.0, y: self.cursor.1 },
                Point { x: self.cursor.0, y: self.cursor.1 },
            ).with_buttons(corrected_buttons);
            
            for hit in targets {
                hit.dispatch(event.clone());
                if event.is_consumed() {
                    break;
                }
            }
            self.mark_dirty();
            true
        } else {
            false
        }
    }
    
    /// Cancels any active gesture, dispatching Cancel events to cached targets.
    /// Call this when:
    /// - Window loses focus
    /// - Mouse leaves window while button pressed
    /// - Any other gesture abort scenario
    pub fn cancel_gesture(&mut self) {
        // Resolve FRESH targets from cached NodeIds
        let targets = self.resolve_hit_path(PointerId::PRIMARY);
        
        // Clear tracker and button state
        self.hit_path_tracker.clear();
        self.buttons_pressed = PointerButtons::NONE;
        
        if !targets.is_empty() {
            let event = PointerEvent::new(
                PointerEventKind::Cancel,
                Point { x: self.cursor.0, y: self.cursor.1 },
                Point { x: self.cursor.0, y: self.cursor.1 },
            );
            
            for hit in targets {
                hit.dispatch(event.clone());
            }
            self.mark_dirty();
        }
    }

    pub fn log_debug_info(&mut self) {
        println!("\n\n");
        println!("════════════════════════════════════════════════════════");
        println!("           DEBUG: CURRENT SCREEN STATE");
        println!("════════════════════════════════════════════════════════");

        if let Some(ref layout_tree) = self.layout_tree {
            log_layout_tree(layout_tree);
            let renderer = HeadlessRenderer::new();
            let render_scene = renderer.render(layout_tree);
            log_render_scene(&render_scene);
            log_screen_summary(layout_tree, &render_scene);
        } else {
            println!("No layout available");
        }

        println!("════════════════════════════════════════════════════════");
        println!("\n\n");
    }

    /// Get the current layout tree (for robot/testing)
    pub fn layout_tree(&self) -> Option<&LayoutTree> {
        self.layout_tree.as_ref()
    }

    /// Get the current semantics tree (for robot/testing)
    pub fn semantics_tree(&self) -> Option<&SemanticsTree> {
        self.semantics_tree.as_ref()
    }

    fn process_frame(&mut self) {
        self.run_layout_phase();
        self.run_dispatch_queues();
        self.run_render_phase();
    }

    fn run_layout_phase(&mut self) {
        // ═══════════════════════════════════════════════════════════════════════════════
        // SCOPED LAYOUT REPASSES (preferred path for local changes)
        // ═══════════════════════════════════════════════════════════════════════════════
        // Process node-specific layout invalidations (e.g., from scroll).
        // This bubbles dirty flags up from specific nodes WITHOUT invalidating all caches.
        // Result: O(subtree) remeasurement, not O(app).
        let repass_nodes = compose_ui::take_layout_repass_nodes();
        if !repass_nodes.is_empty() {
            let mut applier = self.composition.applier_mut();
            for node_id in repass_nodes {
                compose_core::bubble_layout_dirty(&mut *applier as &mut dyn compose_core::Applier, node_id);
            }
            drop(applier);
            self.layout_dirty = true;
        }
        
        // ═══════════════════════════════════════════════════════════════════════════════
        // GLOBAL LAYOUT INVALIDATION (rare fallback for true global events)
        // ═══════════════════════════════════════════════════════════════════════════════
        // This is the "nuclear option" - invalidates ALL layout caches across the entire app.
        //
        // WHEN THIS SHOULD FIRE:
        //   ✓ Window/viewport resize
        //   ✓ Global font scale or density changes
        //   ✓ Debug toggles that affect layout globally
        //
        // WHEN THIS SHOULD *NOT* FIRE:
        //   ✗ Scroll (use schedule_layout_repass instead)
        //   ✗ Single widget updates (use schedule_layout_repass instead)
        //   ✗ Any local layout change (use schedule_layout_repass instead)
        //
        // If you see this firing frequently during normal interactions,
        // someone is abusing request_layout_invalidation() - investigate!
        let invalidation_requested = take_layout_invalidation();
        let did_scoped_repass = self.layout_dirty; // True if we just processed repass_nodes above
        
        if invalidation_requested && !did_scoped_repass {

            // Invalidate all caches (O(app size) - expensive!)
            // This is internal-only API, only accessible via the internal path
            compose_ui::layout::invalidate_all_layout_caches();
            
            // Mark root as needing layout so tree_needs_layout() returns true
            if let Some(root) = self.composition.root() {
                let mut applier = self.composition.applier_mut();
                if let Ok(node) = applier.get_mut(root) {
                    if let Some(layout_node) = node.as_any_mut().downcast_mut::<compose_ui::LayoutNode>() {
                        layout_node.mark_needs_layout();
                    }
                }
            }
            self.layout_dirty = true;
        }
        
        // Early exit if layout is not needed (viewport didn't change, etc.)
        if !self.layout_dirty {
            return;
        }



        let viewport_size = Size {
            width: self.viewport.0,
            height: self.viewport.1,
        };
        if let Some(root) = self.composition.root() {
            let handle = self.composition.runtime_handle();
            let mut applier = self.composition.applier_mut();
            applier.set_runtime_handle(handle);

            // Selective measure optimization: skip layout if tree is clean (O(1) check)
            let needs_layout =
                compose_ui::tree_needs_layout(&mut *applier, root).unwrap_or_else(|err| {
                    log::warn!(
                        "Cannot check layout dirty status for root #{}: {}",
                        root,
                        err
                    );
                    true // Assume dirty on error
                });

            if !needs_layout {
                // Tree is clean - skip layout computation and keep cached layout
                log::trace!("Skipping layout: tree is clean");
                self.layout_dirty = false;
                applier.clear_runtime_handle();
                return;
            }

            // Tree needs layout - compute it
            self.layout_dirty = false;
            
            // Ensure slots exist and borrow mutably (handled inside measure_layout via MemoryApplier)
            match compose_ui::measure_layout(&mut applier, root, viewport_size) {
                Ok(measurements) => {
                    self.semantics_tree = Some(measurements.semantics_tree().clone());
                    self.layout_tree = Some(measurements.into_layout_tree());
                    self.scene_dirty = true;
                }
                Err(err) => {
                    log::error!("failed to compute layout: {err}");
                    self.layout_tree = None;
                    self.semantics_tree = None;
                    self.scene_dirty = true;
                }
            }
            applier.clear_runtime_handle();
        } else {
            self.layout_tree = None;
            self.semantics_tree = None;
            self.scene_dirty = true;
            self.layout_dirty = false;
        }
    }

    fn run_dispatch_queues(&mut self) {
        // Process pointer input repasses
        // Similar to Jetpack Compose's pointer input invalidation processing,
        // we service nodes that need pointer input state updates without forcing layout/draw
        if has_pending_pointer_repasses() {
            let mut applier = self.composition.applier_mut();
            process_pointer_repasses(|node_id| {
                // Access the LayoutNode and clear its dirty flag
                let result = applier.with_node::<LayoutNode, _>(node_id, |layout_node| {
                    if layout_node.needs_pointer_pass() {
                        layout_node.clear_needs_pointer_pass();
                        log::trace!("Cleared pointer repass flag for node #{}", node_id);
                    }
                });
                if let Err(err) = result {
                    log::debug!(
                        "Could not process pointer repass for node #{}: {}",
                        node_id,
                        err
                    );
                }
            });
        }

        // Process focus invalidations
        // Mirrors Jetpack Compose's FocusInvalidationManager.invalidateNodes(),
        // processing nodes that need focus state synchronization
        if has_pending_focus_invalidations() {
            let mut applier = self.composition.applier_mut();
            process_focus_invalidations(|node_id| {
                // Access the LayoutNode and clear its dirty flag
                let result = applier.with_node::<LayoutNode, _>(node_id, |layout_node| {
                    if layout_node.needs_focus_sync() {
                        layout_node.clear_needs_focus_sync();
                        log::trace!("Cleared focus sync flag for node #{}", node_id);
                    }
                });
                if let Err(err) = result {
                    log::debug!(
                        "Could not process focus invalidation for node #{}: {}",
                        node_id,
                        err
                    );
                }
            });
        }
    }

    fn run_render_phase(&mut self) {
        let render_dirty = take_render_invalidation();
        let pointer_dirty = take_pointer_invalidation();
        let focus_dirty = take_focus_invalidation();
        if render_dirty || pointer_dirty || focus_dirty {
            self.scene_dirty = true;
        }
        if !self.scene_dirty {
            return;
        }
        self.scene_dirty = false;
        if let Some(layout_tree) = self.layout_tree.as_ref() {
            let viewport_size = Size {
                width: self.viewport.0,
                height: self.viewport.1,
            };
            if let Err(err) = self.renderer.rebuild_scene(layout_tree, viewport_size) {
                log::error!("renderer rebuild failed: {err:?}");
            }
        } else {
            self.renderer.scene_mut().clear();
        }
    }
}

impl<R> Drop for AppShell<R>
where
    R: Renderer,
{
    fn drop(&mut self) {
        self.runtime.clear_frame_waker();
    }
}

pub fn default_root_key() -> Key {
    location_key(file!(), line!(), column!())
}

#[cfg(test)]
#[path = "tests/app_shell_tests.rs"]
mod tests;
