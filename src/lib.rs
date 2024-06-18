//! This crate provides wrappers for [`std::panic::catch_unwind`] that handle the
//! edge case of the caught panic payload itself panicing when dropped.

use std::{
    any::Any,
    mem,
    panic::{catch_unwind, AssertUnwindSafe, UnwindSafe},
    process::abort,
};

/// Invoke the provided closure and catch any unwinding panics that may occur. If the panic
/// payload panics when dropped, abort the process.
///
/// Returns `Some` if no panics were caught and `None` otherwise.
///
/// See [`std::panic::catch_unwind`] for more information.
#[inline]
#[must_use]
pub fn catch_unwind_or_abort<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Option<R> {
    match catch_unwind(f) {
        Ok(ok) => Some(ok),
        Err(err) => {
            drop_or_abort(err);
            None
        }
    }
}

/// Invoke the provided closure and catch any unwinding panics that may occur. If the panic
/// payload panics when dropped, `mem::forget` the new panic payload and return `None`.
///
/// Returns `Some` if no panics were caught and `None` otherwise.
///
/// See [`std::panic::catch_unwind`] for more information.
#[inline]
#[must_use]
pub fn catch_unwind_or_forget<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Option<R> {
    match catch_unwind(f) {
        Ok(ok) => Some(ok),
        Err(err) => {
            drop_or_forget(err);
            None
        }
    }
}

/// Drop a value. If dropping the value results in an unwinding panic, call the provided closure
/// with the panic payload.
#[inline]
pub fn drop_or_else<T, F: FnOnce(Box<dyn Any + Send + 'static>) -> E, E>(
    value: T,
    or_else: F,
) -> Result<(), E> {
    catch_unwind(AssertUnwindSafe(move || mem::drop(value))).map_err(or_else)
}

/// Drop a value. If dropping the value results in an unwinding panic, abort the process.
#[inline]
pub fn drop_or_abort<T>(value: T) {
    let _ = drop_or_else(value, |_err| abort());
}

/// Drop a value. If dropping the value results in an unwinding panic, `mem::forget` the panic payload.
#[inline]
pub fn drop_or_forget<T>(value: T) {
    let _ = drop_or_else(value, mem::forget);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::panic_any;

    fn endless_panic() {
        struct PanicOnDrop;

        impl Drop for PanicOnDrop {
            fn drop(&mut self) {
                panic_any(Self)
            }
        }

        panic_any(PanicOnDrop)
    }

    #[test]
    fn test_catch_unwind_or_forget() {
        assert_eq!(catch_unwind_or_forget(|| "success"), Some("success"));
        assert_eq!(catch_unwind_or_forget(endless_panic), None);
    }
}
