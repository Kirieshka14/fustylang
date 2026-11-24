use std::collections::HashMap;

/// Opaque handle to a GC-managed heap object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GcHandle(pub usize);

/// Configuration for the controllable mark-and-sweep Garbage Collector.
/// Enabled by default so game developers can script freely without manual memory management,
/// while still being able to disable or budget GC during FPS-critical sections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GcConfig {
    /// Whether automatic GC runs when `live_bytes >= threshold_bytes` (default: `true`).
    pub enabled: bool,
    /// Allocation byte threshold that triggers automatic collection (default: 64 KiB).
    pub threshold_bytes: usize,
    /// Multiplier applied to `threshold_bytes` relative to surviving bytes after a collection.
    pub growth_factor: f64,
    /// Default object budget per incremental `step()` call.
    pub step_budget_objects: usize,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_bytes: 64 * 1024,
            growth_factor: 2.0,
            step_budget_objects: 64,
        }
    }
}

/// Runtime telemetry for the FustyLang Garbage Collector.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GcStats {
    pub enabled: bool,
    pub live_objects: usize,
    pub live_bytes: usize,
    pub total_allocated: usize,
    pub total_freed: usize,
    pub collections: usize,
    pub incremental_steps: usize,
    pub threshold_bytes: usize,
}

/// Payload stored inside a GC-managed object slot.
#[derive(Debug, Clone)]
pub struct GcObject {
    pub tag: &'static str,
    pub size_bytes: usize,
    pub children: Vec<GcHandle>,
    marked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GcPhase {
    Idle,
    Marking,
    Sweeping,
}

/// Controllable mark-and-sweep Garbage Collector designed to coexist with `FrameArena`.
///
/// - **Enabled by default** (`GcConfig::default().enabled == true`).
/// - Supports full `collect(&roots)` and incremental budgeted `step(&roots, budget)`.
/// - Handles cyclic object references cleanly via graph mark-and-sweep.
pub struct GcHeap {
    config: GcConfig,
    next_id: usize,
    objects: HashMap<usize, GcObject>,
    live_bytes: usize,
    total_allocated: usize,
    total_freed: usize,
    collections: usize,
    incremental_steps: usize,
    phase: GcPhase,
    gray_stack: Vec<GcHandle>,
    sweep_cursor: Vec<usize>,
}

impl Default for GcHeap {
    fn default() -> Self {
        Self::new()
    }
}

impl GcHeap {
    pub fn new() -> Self {
        Self::with_config(GcConfig::default())
    }

    pub fn with_config(config: GcConfig) -> Self {
        Self {
            config,
            next_id: 1,
            objects: HashMap::new(),
            live_bytes: 0,
            total_allocated: 0,
            total_freed: 0,
            collections: 0,
            incremental_steps: 0,
            phase: GcPhase::Idle,
            gray_stack: Vec::new(),
            sweep_cursor: Vec::new(),
        }
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
    }

    #[inline]
    pub fn enable(&mut self) {
        self.config.enabled = true;
    }

    #[inline]
    pub fn disable(&mut self) {
        self.config.enabled = false;
    }

    #[inline]
    pub fn set_threshold(&mut self, bytes: usize) {
        self.config.threshold_bytes = bytes.max(256);
    }

    #[inline]
    pub fn config(&self) -> GcConfig {
        self.config
    }

    /// Returns `true` when GC is enabled and live heap bytes reached the threshold.
    #[inline]
    pub fn should_collect(&self) -> bool {
        self.config.enabled && self.live_bytes >= self.config.threshold_bytes
    }

    /// Allocates a new GC-tracked object with the given byte weight and child references.
    pub fn alloc(&mut self, tag: &'static str, size_bytes: usize, children: Vec<GcHandle>) -> GcHandle {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        let weight = size_bytes.max(1);
        self.objects.insert(
            id,
            GcObject {
                tag,
                size_bytes: weight,
                children,
                marked: false,
            },
        );
        self.live_bytes = self.live_bytes.saturating_add(weight);
        self.total_allocated = self.total_allocated.saturating_add(1);
        GcHandle(id)
    }

    /// Adds a child reference edge `parent -> child` (supports cyclic graphs).
    pub fn add_edge(&mut self, parent: GcHandle, child: GcHandle) -> bool {
        if let Some(obj) = self.objects.get_mut(&parent.0) {
            obj.children.push(child);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn contains(&self, handle: GcHandle) -> bool {
        self.objects.contains_key(&handle.0)
    }

    #[inline]
    pub fn get(&self, handle: GcHandle) -> Option<&GcObject> {
        self.objects.get(&handle.0)
    }

    /// Runs a full mark-and-sweep collection from `roots` and returns the number of freed objects.
    pub fn collect(&mut self, roots: &[GcHandle]) -> usize {
        self.phase = GcPhase::Idle;
        self.gray_stack.clear();
        self.sweep_cursor.clear();

        for obj in self.objects.values_mut() {
            obj.marked = false;
        }

        // Mark phase (iterative DFS over reachable object graph)
        for &root in roots {
            if let Some(obj) = self.objects.get_mut(&root.0) {
                if !obj.marked {
                    obj.marked = true;
                    self.gray_stack.push(root);
                }
            }
        }
        while let Some(handle) = self.gray_stack.pop() {
            let children = match self.objects.get(&handle.0) {
                Some(obj) => obj.children.clone(),
                None => continue,
            };
            for child in children {
                if let Some(child_obj) = self.objects.get_mut(&child.0) {
                    if !child_obj.marked {
                        child_obj.marked = true;
                        self.gray_stack.push(child);
                    }
                }
            }
        }

        // Sweep phase
        let mut freed = 0usize;
        let mut freed_bytes = 0usize;
        self.objects.retain(|_, obj| {
            if obj.marked {
                obj.marked = false;
                true
            } else {
                freed += 1;
                freed_bytes += obj.size_bytes;
                false
            }
        });

        self.live_bytes = self.live_bytes.saturating_sub(freed_bytes);
        self.total_freed = self.total_freed.saturating_add(freed);
        self.collections = self.collections.saturating_add(1);
        self.recompute_threshold();
        freed
    }

    /// Performs a budgeted incremental GC step (at most `budget` objects visited).
    /// Returns the number of objects freed during this step.
    pub fn step(&mut self, roots: &[GcHandle], budget: usize) -> usize {
        let mut remaining = budget.max(1);
        let mut freed = 0usize;
        self.incremental_steps = self.incremental_steps.saturating_add(1);

        if self.phase == GcPhase::Idle {
            for obj in self.objects.values_mut() {
                obj.marked = false;
            }
            self.gray_stack.clear();
            for &root in roots {
                if let Some(obj) = self.objects.get_mut(&root.0) {
                    if !obj.marked {
                        obj.marked = true;
                        self.gray_stack.push(root);
                    }
                }
            }
            self.phase = GcPhase::Marking;
        }

        if self.phase == GcPhase::Marking {
            while remaining > 0 {
                let Some(handle) = self.gray_stack.pop() else {
                    self.sweep_cursor = self.objects.keys().copied().collect();
                    self.phase = GcPhase::Sweeping;
                    break;
                };
                remaining -= 1;
                let children = match self.objects.get(&handle.0) {
                    Some(obj) => obj.children.clone(),
                    None => continue,
                };
                for child in children {
                    if let Some(child_obj) = self.objects.get_mut(&child.0) {
                        if !child_obj.marked {
                            child_obj.marked = true;
                            self.gray_stack.push(child);
                        }
                    }
                }
            }
        }

        if self.phase == GcPhase::Sweeping {
            while remaining > 0 {
                let Some(id) = self.sweep_cursor.pop() else {
                    self.phase = GcPhase::Idle;
                    self.collections = self.collections.saturating_add(1);
                    self.recompute_threshold();
                    break;
                };
                remaining -= 1;
                let should_remove = self
                    .objects
                    .get_mut(&id)
                    .map(|obj| {
                        if obj.marked {
                            obj.marked = false;
                            false
                        } else {
                            true
                        }
                    })
                    .unwrap_or(false);
                if should_remove {
                    if let Some(removed) = self.objects.remove(&id) {
                        self.live_bytes = self.live_bytes.saturating_sub(removed.size_bytes);
                        self.total_freed = self.total_freed.saturating_add(1);
                        freed += 1;
                    }
                }
            }
        }

        freed
    }

    fn recompute_threshold(&mut self) {
        let min_threshold = 4096usize;
        let scaled = ((self.live_bytes as f64) * self.config.growth_factor.max(1.25)) as usize;
        self.config.threshold_bytes = scaled.max(min_threshold);
    }

    pub fn stats(&self) -> GcStats {
        GcStats {
            enabled: self.config.enabled,
            live_objects: self.objects.len(),
            live_bytes: self.live_bytes,
            total_allocated: self.total_allocated,
            total_freed: self.total_freed,
            collections: self.collections,
            incremental_steps: self.incremental_steps,
            threshold_bytes: self.config.threshold_bytes,
        }
    }

    pub fn clear(&mut self) {
        self.objects.clear();
        self.live_bytes = 0;
        self.phase = GcPhase::Idle;
        self.gray_stack.clear();
        self.sweep_cursor.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gc_enabled_by_default_and_toggleable() {
        let mut gc = GcHeap::new();
        assert!(gc.is_enabled());
        gc.disable();
        assert!(!gc.is_enabled());
        gc.enable();
        assert!(gc.is_enabled());
    }

    #[test]
    fn test_gc_collects_unreachable_cycles_and_preserves_roots() {
        let mut gc = GcHeap::new();
        let root = gc.alloc("root", 64, vec![]);
        let child = gc.alloc("child", 32, vec![]);
        gc.add_edge(root, child);

        // Unreachable cycle A <-> B
        let cyc_a = gc.alloc("cycle_a", 128, vec![]);
        let cyc_b = gc.alloc("cycle_b", 128, vec![]);
        gc.add_edge(cyc_a, cyc_b);
        gc.add_edge(cyc_b, cyc_a);

        let freed = gc.collect(&[root]);
        assert_eq!(freed, 2);
        assert!(gc.contains(root));
        assert!(gc.contains(child));
        assert!(!gc.contains(cyc_a));
        assert!(!gc.contains(cyc_b));
        assert_eq!(gc.stats().live_objects, 2);
        assert_eq!(gc.stats().live_bytes, 96);
    }

    #[test]
    fn test_gc_incremental_step_completes_collection() {
        let mut gc = GcHeap::new();
        let root = gc.alloc("root", 16, vec![]);
        for _ in 0..20 {
            gc.alloc("temp", 32, vec![]);
        }
        let mut total_freed = 0;
        for _ in 0..10 {
            total_freed += gc.step(&[root], 5);
        }
        assert_eq!(total_freed, 20);
        assert_eq!(gc.stats().live_objects, 1);
        assert!(gc.stats().incremental_steps >= 1);
    }
}
