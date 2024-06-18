# catch-unwind

This crate provides wrappers for `std::panic::catch_unwind` that handle the
edge case of the caught panic payload itself panicing when dropped.

See the documentation at https://docs.rs/catch-unwind.

### Version history

- 0.1.1 - Added `catch_unwind_with_or_abort` and `catch_unwind_with_or_forget`
- 0.1.0 - Initial release
