use std::{
    alloc::{Allocator, Global},
    panic::Location,
    sync::{
        atomic::{AtomicIsize, Ordering},
        Arc,
    },
};

pub type UnkaiGlobal = Unkai<Global>;

/// Entrypoint to use with [`Allocator`]. Example usage:
///
/// ```rust
/// # #![feature(allocator_api)]
/// # use std::alloc::Global;
/// # use unkai::{UnkaiGlobal, Unkai};
/// let mut vec_container: Vec<usize, UnkaiGlobal> = Vec::with_capacity_in(10000, Unkai::default());
/// assert_eq!(vec_container.allocator().report_usage(), 80000);
/// ```
///
/// There is also an example file `examples/allocator.rs` that shows more usages.
pub struct Unkai<A>
where
    A: Allocator,
{
    caller: &'static Location<'static>,
    usage: Arc<AtomicIsize>,
    alloc: A,
}

impl<A> Unkai<A>
where
    A: Allocator,
{
    /// Create a [`Unkai`] allocator with given [`Allocator`] impl.
    ///
    /// [`Unkai`] will remember the code place where it was created. This information
    /// can be retrieved with [`Unkai::report_caller`] method.
    #[track_caller]
    pub fn new(alloc: A) -> Self {
        Self {
            caller: Location::caller(),
            usage: Arc::new(AtomicIsize::new(0)),
            alloc,
        }
    }

    /// Get current allocated memory in bytes.
    ///
    /// When [`Unkai`] is [`Clone`]-d, the memory usage is shared between
    /// two instances. If this is not the desired behavior, please construct
    /// a new [`Unkai`] instance instead of cloning it.
    pub fn report_usage(&self) -> isize {
        self.usage.load(Ordering::Relaxed)
    }

    /// Get where the [`Unkai`] instance is constructed.
    pub fn report_caller(&self) -> &'static Location<'static> {
        self.caller
    }
}

unsafe impl<A> Allocator for Unkai<A>
where
    A: Allocator,
{
    fn allocate(
        &self,
        layout: std::alloc::Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
        let size = layout.size();
        self.usage.fetch_add(size as isize, Ordering::Relaxed);

        self.alloc.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, layout: std::alloc::Layout) {
        let size = layout.size();
        self.usage.fetch_sub(size as isize, Ordering::Relaxed);

        self.alloc.deallocate(ptr, layout)
    }
}

impl<A: Allocator + Clone> Clone for Unkai<A> {
    fn clone(&self) -> Self {
        Self {
            caller: self.caller,
            usage: self.usage.clone(),
            alloc: self.alloc.clone(),
        }
    }
}

impl Default for UnkaiGlobal {
    #[track_caller]
    fn default() -> Self {
        Self {
            caller: Location::caller(),
            usage: Default::default(),
            alloc: Global,
        }
    }
}
