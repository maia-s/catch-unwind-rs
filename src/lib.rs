//! This crate provides wrappers for [`std::panic::catch_unwind`] that handle the
//! edge case of the caught panic payload itself panicing when dropped.

use std::{
    any::Any,
    mem,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe, UnwindSafe},
    process::abort,
};

/// What to do with a caught payload
pub enum PayloadAction {
    /// Drop the payload and return, or abort if it panics on drop
    DropOrAbort,

    /// Drop the payload and return, or forget the new payload if it panics on drop
    DropOrForget,

    /// Drop the payload and return, without guarding against panic on drop
    DropOrUnwind,

    /// Resume unwinding
    ResumeUnwind,
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

/// Invoke the provided closure and catch any unwinding panics that may occur.
/// You can inspect the payload before it's dropped with the `inspect` closure, and
/// choose what to do with it using [`PayloadAction`].
///
/// If `inspect` panics, abort the process.
///
/// Returns `Some` if no panics were caught or `None` if panics were caught and dropped.
///
/// See [`std::panic::catch_unwind`] for more information.
#[inline]
#[must_use]
pub fn catch_unwind_with<
    F: FnOnce() -> R + UnwindSafe,
    R,
    I: FnOnce(&(dyn Any + Send + 'static)) -> PayloadAction + UnwindSafe,
>(
    f: F,
    inspect: I,
) -> Option<R> {
    match catch_unwind(f) {
        Ok(ok) => Some(ok),
        Err(err) => match catch_unwind(AssertUnwindSafe(|| inspect(&err))) {
            Ok(PayloadAction::ResumeUnwind) => resume_unwind(err),
            Ok(PayloadAction::DropOrAbort) => {
                drop_or_abort(err);
                None
            }
            Ok(PayloadAction::DropOrForget) => {
                drop_or_forget(err);
                None
            }
            Ok(PayloadAction::DropOrUnwind) => {
                mem::drop(err);
                None
            }
            Err(_err2) => {
                abort();
            }
        },
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

    #[test]
    fn test_catch_unwind_with() {
        let mut count = 0;
        let count_ref = AssertUnwindSafe(&mut count);
        assert_eq!(
            catch_unwind_with(
                || "success",
                move |_| {
                    let c = count_ref;
                    *c.0 += 1;
                    PayloadAction::DropOrForget
                }
            ),
            Some("success")
        );
        assert_eq!(count, 0);

        let count_ref = AssertUnwindSafe(&mut count);
        assert_eq!(
            catch_unwind_with(endless_panic, move |_| {
                let c = count_ref;
                *c.0 += 1;
                PayloadAction::DropOrForget
            }),
            None
        );
        assert_eq!(count, 1);

        match catch_unwind(|| catch_unwind_with(endless_panic, |_| PayloadAction::ResumeUnwind)) {
            Ok(_) => panic!("Caught::ResumeUnwind didn't resume"),
            Err(err) => mem::forget(err),
        }
    }
}
