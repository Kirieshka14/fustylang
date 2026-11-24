use crate::JitError;

#[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "macos")))]
mod sys {
    use std::ffi::c_void;

    pub const PROT_READ: i32 = 0x1;
    pub const PROT_WRITE: i32 = 0x2;
    pub const PROT_EXEC: i32 = 0x4;
    pub const MAP_PRIVATE: i32 = 0x02;
    #[cfg(target_os = "linux")]
    pub const MAP_ANONYMOUS: i32 = 0x20;
    #[cfg(target_os = "macos")]
    pub const MAP_ANONYMOUS: i32 = 0x1000;
    #[cfg(target_os = "macos")]
    pub const MAP_JIT: i32 = 0x0800;
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

#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
mod win {
    use std::ffi::c_void;

    pub const MEM_COMMIT: u32 = 0x1000;
    pub const MEM_RESERVE: u32 = 0x2000;
    pub const MEM_RELEASE: u32 = 0x8000;
    pub const PAGE_READWRITE: u32 = 0x04;
    pub const PAGE_EXECUTE_READ: u32 = 0x20;

    #[link(name = "kernel32")]
    extern "system" {
        pub fn VirtualAlloc(
            lp_address: *mut c_void,
            dw_size: usize,
            fl_allocation_type: u32,
            fl_protect: u32,
        ) -> *mut c_void;
        pub fn VirtualProtect(
            lp_address: *mut c_void,
            dw_size: usize,
            fl_new_protect: u32,
            lpfl_old_protect: *mut u32,
        ) -> i32;
        pub fn VirtualFree(lp_address: *mut c_void, dw_size: usize, dw_free_type: u32) -> i32;
    }
}

/// Page-aligned W^X executable memory buffer for JIT-compiled native x86-64 code
/// (supports Linux, macOS, and Windows x86-64 with zero external dependencies).
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

    /// Allocates RW memory, copies `code`, and seals it as RX (W^X) on Linux or macOS x86-64.
    #[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "macos")))]
    pub fn from_code(code: &[u8]) -> Result<Self, JitError> {
        if code.is_empty() {
            return Err(JitError::new(
                "B-baka! Cannot allocate an empty executable JIT buffer! >.<",
            ));
        }

        let capacity = code.len().div_ceil(Self::PAGE_SIZE) * Self::PAGE_SIZE;

        #[cfg(target_os = "linux")]
        let flags = sys::MAP_PRIVATE | sys::MAP_ANONYMOUS;
        #[cfg(target_os = "macos")]
        let flags = sys::MAP_PRIVATE | sys::MAP_ANONYMOUS | sys::MAP_JIT;

        let ptr = unsafe {
            sys::mmap(
                std::ptr::null_mut(),
                capacity,
                sys::PROT_READ | sys::PROT_WRITE,
                flags,
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

    /// Allocates RW memory via `VirtualAlloc`, copies `code`, and seals it as RX via `VirtualProtect` on Windows x86-64.
    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    pub fn from_code(code: &[u8]) -> Result<Self, JitError> {
        if code.is_empty() {
            return Err(JitError::new(
                "B-baka! Cannot allocate an empty executable JIT buffer! >.<",
            ));
        }

        let capacity = code.len().div_ceil(Self::PAGE_SIZE) * Self::PAGE_SIZE;

        let ptr = unsafe {
            win::VirtualAlloc(
                std::ptr::null_mut(),
                capacity,
                win::MEM_COMMIT | win::MEM_RESERVE,
                win::PAGE_READWRITE,
            )
        };

        if ptr.is_null() {
            return Err(JitError::new(
                "Hmph! VirtualAlloc failed to allocate JIT code page! >///<",
            ));
        }

        unsafe {
            std::ptr::copy_nonoverlapping(code.as_ptr(), ptr as *mut u8, code.len());
            let mut old_protect: u32 = 0;
            if win::VirtualProtect(ptr, capacity, win::PAGE_EXECUTE_READ, &mut old_protect) == 0 {
                win::VirtualFree(ptr, 0, win::MEM_RELEASE);
                return Err(JitError::new(
                    "B-baka! VirtualProtect failed to seal JIT page as W^X (RX)! >///<",
                ));
            }
        }

        Ok(Self {
            ptr: ptr as *mut u8,
            capacity,
            len: code.len(),
        })
    }

    #[cfg(not(all(
        target_arch = "x86_64",
        any(target_os = "linux", target_os = "windows", target_os = "macos")
    )))]
    pub fn from_code(_code: &[u8]) -> Result<Self, JitError> {
        Err(JitError::new(
            "Native x86-64 JIT is not available on this platform; falling back to VM.",
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
        #[cfg(all(target_arch = "x86_64", any(target_os = "linux", target_os = "macos")))]
        unsafe {
            if !self.ptr.is_null() && self.capacity > 0 {
                sys::munmap(self.ptr as *mut std::ffi::c_void, self.capacity);
            }
        }

        #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
        unsafe {
            if !self.ptr.is_null() && self.capacity > 0 {
                win::VirtualFree(self.ptr as *mut std::ffi::c_void, 0, win::MEM_RELEASE);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(
        target_arch = "x86_64",
        any(target_os = "linux", target_os = "windows", target_os = "macos")
    ))]
    fn test_wx_executable_buffer_runs_native_code() {
        // x86-64: mov eax, 42; ret
        let code: [u8; 6] = [0xB8, 42, 0x00, 0x00, 0x00, 0xC3];
        let buf = ExecutableBuffer::from_code(&code).expect("W^X executable allocation failed");
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
