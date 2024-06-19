//! This crate provides wrappers for [`std::panic::catch_unwind`] that handle the
//! edge case of the caught panic payload itself panicing when dropped.

use std::{
    any::Any,
    mem,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe, UnwindSafe},
    process::abort,
};

/// Unwinding payload wrapped to abort by default if it panics on drop
pub struct Payload(Option<Box<dyn Any + Send + 'static>>);

impl Payload {
    /// Get a reference to the payload
    #[inline]
    pub fn get(&self) -> &(dyn Any + Send + 'static) {
        let Some(payload) = &self.0 else {
            unreachable!()
        };
        payload
    }

    /// Get a mutable reference to the payload
    #[inline]
    pub fn get_mut(&mut self) -> &mut (dyn Any + Send + 'static) {
        let Some(payload) = &mut self.0 else {
            unreachable!()
        };
        payload
    }

    /// Get the payload itself. This may panic when dropped
    #[inline]
    pub fn into_inner(mut self) -> Box<dyn Any + Send + 'static> {
        self.0.take().unwrap()
    }

    /// Drop the payload and abort the process if doing so panics
    #[inline]
    pub fn drop_or_abort(self) {
        drop_or_abort(self.into_inner())
    }

    /// Drop the payload. If doing so panics, `mem::forget` the new payload
    #[inline]
    pub fn drop_or_forget(self) {
        drop_or_forget(self.into_inner())
    }

    /// Resume unwinding with this payload
    #[inline]
    pub fn resume_unwind(self) {
        resume_unwind(self.into_inner())
    }
}

impl Drop for Payload {
    #[inline]
    fn drop(&mut self) {
        if let Some(payload) = self.0.take() {
            drop_or_abort(payload)
        }
    }
}

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

/// Invoke the provided closure and catch any unwinding panics that may occur. This wraps
/// the unwinding payload in [`Payload`], which will abort if it panics on drop by default.
/// You can use the methods of `Payload` to change this behaviour.
///
/// Returns `Ok` if no panics were caught and `Err(Payload)` otherwise.
///
/// See [`std::panic::catch_unwind`] for more information.
#[inline]
pub fn catch_unwind_wrapped<F: FnOnce() -> R + UnwindSafe, R>(f: F) -> Result<R, Payload> {
    catch_unwind(f).map_err(|e| Payload(Some(e)))
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

    #[test]
    fn test_catch_unwind_wrapped() {
        assert!(matches!(catch_unwind_wrapped(|| "success"), Ok("success")));

        match catch_unwind(|| match catch_unwind_wrapped(endless_panic) {
            Ok(()) => unreachable!(),
            Err(payload) => payload.drop_or_forget(),
        }) {
            Ok(()) => (),
            Err(_) => panic!("Payload::drop_or_forget didn't forget"),
        }

        match catch_unwind(|| match catch_unwind_wrapped(endless_panic) {
            Ok(()) => unreachable!(),
            Err(payload) => payload.resume_unwind(),
        }) {
            Ok(()) => panic!("Payload::resume_unwind didn't resume"),
            Err(err) => drop_or_forget(err),
        }
    }
}
