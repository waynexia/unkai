use std::{
    alloc::Allocator,
    sync::atomic::{AtomicIsize, Ordering},
};

/// Entrypoint to use with [`Allocator`].
pub struct Unkai<A>
where
    A: Allocator,
{
    file_path: String,
    line_num: u32,
    usage: AtomicIsize,
    alloc: A,
}

impl<A> Unkai<A>
where
    A: Allocator,
{
    fn new<S: AsRef<String>>(file_path: S, line_num: u32, alloc: A) -> Self {
        Self {
            file_path: file_path.as_ref().to_owned(),
            line_num,
            usage: AtomicIsize::new(0),
            alloc,
        }
    }

    fn report(&self) -> (String, isize) {
        let caller = format!("{}:{}", self.file_path, self.line_num);
        let usage = self.usage.load(Ordering::Relaxed);

        (caller, usage)
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
