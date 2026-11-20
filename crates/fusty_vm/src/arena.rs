/// Fast Bump Arena allocator for frame-scoped allocations in Ferrium
pub struct FrameArena {
    buffer: Vec<u8>,
    offset: usize,
}

impl FrameArena {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0; capacity],
            offset: 0,
        }
    }

    /// Reset arena offset to 0 at the end of each frame (O(1) deallocation)
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    pub fn used_bytes(&self) -> usize {
        self.offset
    }

    /// Allocate an aligned raw slice of bytes from the frame arena.
    pub fn alloc_bytes(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        if !align.is_power_of_two() {
            return None;
        }
        let base_ptr = self.buffer.as_mut_ptr();
        let cur_ptr = base_ptr.wrapping_add(self.offset);
        let pad = cur_ptr.align_offset(align);
        if pad == usize::MAX {
            return None;
        }
        let aligned = self.offset.checked_add(pad)?;
        let end = aligned.checked_add(size)?;
        if end <= self.buffer.len() {
            self.offset = end;
            // SAFETY: `aligned <= end <= self.buffer.len()` and buffer is uniquely borrowed.
            Some(unsafe { base_ptr.add(aligned) })
        } else {
            None
        }
    }
}
