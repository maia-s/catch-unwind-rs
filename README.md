# catch-unwind

This crate provides wrappers for `std::panic::catch_unwind` that handle the
edge case of the caught panic payload itself panicing when dropped.

See the documentation at https://docs.rs/catch-unwind.
