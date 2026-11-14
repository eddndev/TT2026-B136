//! Immediate termination for the private worker processes.

/// Ends every thread of the worker at once. No atexit handler, stdio flush or
/// instrumentation writer runs after the response has been written.
pub(crate) fn exit_group(code: i32) -> ! {
    // SAFETY: `_exit` only issues the exit_group system call; it runs no Rust
    // destructors or native handlers that could observe borrowed state.
    unsafe { libc::_exit(code) }
}
