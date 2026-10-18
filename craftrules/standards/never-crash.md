# Never crash

Outranks feature work. A bad input — malformed file, pulled drive, hostile JSON, full disk —
must surface as an error the user can act on, never a panic.

- **Non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`,
  `todo!`, `unimplemented!`. Return the crate's error type and propagate with `?`.
- Every production crate root (`lib.rs`, each binary's `main.rs`, `xtask`) carries:

  ```rust
  #![deny(
      clippy::unwrap_used, clippy::expect_used, clippy::panic,
      clippy::unimplemented, clippy::todo, clippy::unreachable
  )]
  ```

  Tests may unwrap — `clippy.toml` sets `allow-unwrap-in-tests = true`.
- **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`. Reach the OS through
  documented interfaces (/sys, /proc, plain file I/O, portals), never ioctls or FFI.
- **Everything from the user, the OS or the network is hostile.** `get()` instead of
  indexing, checked arithmetic for sizes and offsets, cap sizes read from input, guard
  division by zero. A half-written artifact the user asked for beats a panic they didn't.
