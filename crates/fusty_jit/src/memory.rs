use crate::JitError;

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
mod sys {
    use std::ffi::c_void;

    pub const PROT_READ: i32 = 0x1;
    pub const PROT_WRITE: i32 = 0x2;
    pub const PROT_EXEC: i32 = 0x4;
    pub const MAP_PRIVATE: i32 = 0x02;
    pub const MAP_ANONYMOUS: i32 = 0x20;
    pub const MAP_FAILED: *mut c_void = !0usize as *mut c_void;

    extern "C" {
        pub fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: i32,
            flags: i32,
            fd: i32,
            offset: isize,
        ) -> *mut c_void;
        pub fn mprotect(addr: *mut c_void, len: usize, prot: i32) -> i32;
        pub fn munmap(addr: *mut c_void, len: usize) -> i32;
    }
}

/// Page-aligned W^X executable memory buffer for JIT-compiled native x86-64 code.
pub struct ExecutableBuffer {
    ptr: *mut u8,
    capacity: usize,
    len: usize,
}

// ExecutableBuffer owns immutable RX memory once sealed and has no interior mutability.
unsafe impl Send for ExecutableBuffer {}
unsafe impl Sync for ExecutableBuffer {}

impl ExecutableBuffer {
    const PAGE_SIZE: usize = 4096;

    /// Allocates RW memory, copies `code`, and seals it as RX (W^X).
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    pub fn from_code(code: &[u8]) -> Result<Self, JitError> {
        if code.is_empty() {
            return Err(JitError::new(
                "B-baka! Cannot allocate an empty executable JIT buffer! >.<",
            ));
        }

        let capacity = code.len().div_ceil(Self::PAGE_SIZE) * Self::PAGE_SIZE;

        let ptr = unsafe {
            sys::mmap(
                std::ptr::null_mut(),
                capacity,
                sys::PROT_READ | sys::PROT_WRITE,
                sys::MAP_PRIVATE | sys::MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if ptr == sys::MAP_FAILED || ptr.is_null() {
            return Err(JitError::new(
                "Hmph! mmap failed to allocate JIT code page! >///<",
            ));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());
            if sys::mprotect(ptr, capacity, sys::PROT_READ | sys::PROT_EXEC) != 0 {
                sys::munmap(ptr, capacity);
                return Err(JitError::new(
                    "B-baka! mprotect failed to seal JIT page as W^X (RX)! >///<",
                ));
            }
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            capacity,
            len: code.len(),
        })
    }

    #[cfg(not(all(target_arch = "x86_64", target_os = "linux")))]
    pub fn from_code(_code: &[u8]) -> Result<Self, JitError> {
        Err(JitError::new(
            "Native x86-64 Linux JIT is not available on this platform; falling back to VM.",
        ))
    }

    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for ExecutableBuffer {
    fn drop(&mut self) {
        #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
        unsafe {
            if !self.ptr.is_null() && self.capacity > 0 {
                sys::munmap(self.ptr as *mut std::ffi::c_void, self.capacity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    fn test_wx_executable_buffer_runs_native_code() {
        // x86-64: mov eax, 42; ret
        let code: [u8; 6] = [0xB8, 42, 0x00, 0x00, 0x00, 0xC3];
        let buf = ExecutableBuffer::from_code(&code).expect("mmap + mprotect failed");
        assert_eq!(buf.len(), 6);
        assert!(buf.capacity() >= 4096);

        let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(buf.as_ptr()) };
        assert_eq!(func(), 42);
    }

    #[test]
    fn test_empty_code_rejected() {
        assert!(ExecutableBuffer::from_code(&[]).is_err());
    }
}
