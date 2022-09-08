#![feature(allocator_api)]

use once_cell::sync::OnceCell;
use std::fmt::Write;
use std::{
    alloc::{Allocator, GlobalAlloc},
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
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
    disabled: AtomicBool,
}

impl<A> UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    const SKIP_DEPTH: usize = 5;
    const FETCH_DEPTH: usize = 10;

    pub const fn new(alloc: A) -> Self {
        let frame_counter = OnceCell::<DashMap<Vec<usize>, AtomicIsize>>::new();
        Self {
            alloc,
            frame_counter,
            disabled: AtomicBool::new(false),
        }
    }

    pub fn report_addr(&self) -> Vec<(Vec<usize>, isize)> {
        self.disabled.store(true, Ordering::Relaxed);
        let res = self
            .frame_counter
            .get_or_init(DashMap::new)
            .iter()
            .map(|item| (item.key().clone(), item.value().load(Ordering::Relaxed)))
            .collect();
        self.disabled.store(false, Ordering::Relaxed);

        res
    }

    pub fn report_symbol(&self) -> Vec<(String, isize)> {
        self.disabled.store(true, Ordering::Relaxed);
        let res = self
            .frame_counter
            .get_or_init(DashMap::new)
            .iter()
            .map(|item| {
                let ips = item.key();
                let mut stack = String::new();
                for ip in ips {
                    backtrace::resolve((*ip) as _, |symbol| {
                        if let Some(file_name) = symbol.filename() {
                            let _ = write!(stack, "{}", file_name.display());
                        }
                        if let Some(line_num) = symbol.lineno() {
                            let _ = write!(stack, ":{:?}", line_num);
                        }
                        if let Some(name) = symbol.name() {
                            let _ = write!(stack, " @ {:?}", name);
                        }
                        let _ = writeln!(stack);
                    });
                }

                (stack, item.value().load(Ordering::Relaxed))
            })
            .collect();
        self.disabled.store(false, Ordering::Relaxed);

        res
    }
}

unsafe impl<A> GlobalAlloc for UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = self.alloc.alloc(layout);
        if !self.disabled.load(Ordering::Relaxed) && sampling(ptr) {
            let frames = partial_trace(Self::SKIP_DEPTH, Self::FETCH_DEPTH);
            self.frame_counter
                .get_or_init(DashMap::new)
                .entry(frames)
                .or_default()
                .fetch_add(layout.size() as isize, Ordering::Relaxed);
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        if !self.disabled.load(Ordering::Relaxed) && sampling(ptr) {
            let frames = partial_trace(Self::SKIP_DEPTH, Self::FETCH_DEPTH);
            self.frame_counter
                .get_or_init(DashMap::new)
                .entry(frames)
                .or_default()
                .fetch_sub(layout.size() as isize, Ordering::Relaxed);
        }

        self.alloc.dealloc(ptr, layout)
    }
}

fn partial_trace(skip: usize, fetch: usize) -> Vec<usize> {
    let mut skipped = 0;
    let mut fetched = 0;
    let mut res = Vec::with_capacity(fetch);

    backtrace::trace(|frame| {
        if skipped < skip {
            skipped += 1;
            return true;
        }

        let ip = frame.ip() as usize;
        res.push(ip);

        fetched += 1;
        fetched < fetch
    });

    res
}

fn sampling(ptr: *mut u8) -> bool {
    ((ptr as usize) >> 3) % 99 == 0
}
