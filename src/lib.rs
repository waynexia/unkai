#![feature(allocator_api)]

use once_cell::sync::OnceCell;
use std::{
    alloc::{Allocator, GlobalAlloc},
    marker::PhantomData,
    sync::atomic::{AtomicIsize, Ordering},
};

use dashmap::DashMap;

pub struct UnkaiRoot<A>
where
    A: Allocator,
{
    _phantom: PhantomData<A>,
}

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

pub struct UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    alloc: A,
    frame_counter: OnceCell<DashMap<Vec<usize>, AtomicIsize>>,
}

impl<A> UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    const TRACE_DEPTH: usize = 5;

    pub const fn new(alloc: A) -> Self {
        let frame_counter = OnceCell::<DashMap<Vec<usize>, AtomicIsize>>::new();
        Self {
            alloc,
            // frame_counter: DashMap::new(),
            frame_counter,
        }
    }

    pub fn report_addr(&self) -> Vec<(Vec<usize>, isize)> {
        self.frame_counter
            .get_or_init(|| DashMap::new())
            .iter()
            .map(|item| (item.key().clone(), item.value().load(Ordering::Relaxed)))
            .collect()
    }

    pub fn report_symbol(&self) -> Vec<(String, isize)> {
        self.frame_counter
            .get_or_init(|| DashMap::new())
            .iter()
            .map(|item| {
                let ips = item.key();
                let mut stack = String::new();
                for ip in ips {
                    backtrace::resolve((*ip) as _, |symbol| {
                        stack += &format!(
                            "{:?}:{:?} @ '{:?}'\n",
                            symbol.filename(),
                            symbol.lineno(),
                            symbol.name()
                        );
                    });
                }

                (stack, item.value().load(Ordering::Relaxed))
            })
            .collect()
    }
}

unsafe impl<A> GlobalAlloc for UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = self.alloc.alloc(layout);
        if sampling(ptr) {
            let frames = partial_trace(Self::TRACE_DEPTH);
            self.frame_counter
                .get_or_init(|| DashMap::new())
                .entry(frames)
                .or_default()
                .fetch_add(layout.size() as isize, Ordering::Relaxed);
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        if sampling(ptr) {
            let frames = partial_trace(Self::TRACE_DEPTH);
            self.frame_counter
                .get_or_init(|| DashMap::new())
                .entry(frames)
                .or_default()
                .fetch_sub(layout.size() as isize, Ordering::Relaxed);
        }

        self.alloc.dealloc(ptr, layout)
    }
}

fn partial_trace(depth: usize) -> Vec<usize> {
    let mut counter = 0;
    let mut res = Vec::with_capacity(depth);

    backtrace::trace(|frame| {
        let ip = frame.ip() as usize;
        res.push(ip);

        counter += 1;
        counter < depth
    });

    res
}

fn sampling(ptr: *mut u8) -> bool {
    ((ptr as usize) >> 3) % 99 == 0
}
