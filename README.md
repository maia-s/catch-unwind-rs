# catch-unwind

This crate provides wrappers for `std::panic::catch_unwind` that handle the
edge case of the caught panic payload itself panicing when dropped.

See the documentation at https://docs.rs/catch-unwind.

### Version history

- 0.3.0 - Added `catch_unwind_wrapped`, removed `catch_unwind_with`
- 0.2.0 - Replace the `with` functions from 0.1.1 with a more general `catch_unwind_with`
- 0.1.1 - Added `catch_unwind_with_or_abort` and `catch_unwind_with_or_forget`
- 0.1.0 - Initial release
