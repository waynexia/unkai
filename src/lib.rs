//! Unkai is a tool set for Rust's memory allocation APIs mainly focus on tracking
//! and conditional analyzing / limiting memory usage.
//!
//! # Basic Usage
//!
//! It's now compatible with two major forms of allocator API in the standard
//! library:
//! - [`GlobalAlloc`] : the global memory allocator for all default memory allocation.
//! - [`Allocator`] : the unstable allocator API ([tracking issue]) that allows
//!   changing [`Allocator`] for a specific struct like `Box` or `Vec`.
//!
//! ## Use with [`GlobalAlloc`]
//!
//! The entrypoint is [`UnkaiGlobalAlloc`]. Only need to wrap your original global
//! allocator with [`UnkaiGlobalAlloc`] like this:
//!
//! ```rust, ignore
//! use tikv_jemallocator::Jemalloc;
//! use unkai::UnkaiGlobalAlloc;
//!
//! #[global_allocator]
//! static UNKAI: UnkaiGlobalAlloc<Jemalloc> = UnkaiGlobalAlloc::new(Jemalloc {}, 99, 5, 10, 0);
//! ```
//!
//! ## Use with [`Allocator`]
//!
//! Notice that [`Allocator`] only available when the unstable feature `allocator_api`
//! is enabled via `#![feature(allocator_api)]`. And enabling unstable feature requires
//! the nigntly channel Rust toolchain.
//!
//! The entrypoint is [`Unkai`]. Example usage:
//!
//! ```rust
//! # #![feature(allocator_api)]
//! # use std::alloc::Global;
//! # use unkai::{UnkaiGlobal, Unkai};
//! let mut vec_container: Vec<usize, UnkaiGlobal> = Vec::with_capacity_in(10000, Unkai::default());
//! assert_eq!(vec_container.allocator().report_usage(), 80000);
//! ```
//!
//! There is also an example file `examples/allocator.rs` that shows more usages.
//!
//! # Tracking allocation
//!
//! TBD
//!
//! [tracking issue]: https://github.com/rust-lang/rust/issues/32838
//! [`Allocator`]: std::alloc::Allocator
//! [`GlobalAlloc`]: std::alloc::GlobalAlloc

#![feature(allocator_api)]

mod allocator;
mod global_alloc;

pub use allocator::{Unkai, UnkaiGlobal};
pub use global_alloc::UnkaiGlobalAlloc;
