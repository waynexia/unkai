use once_cell::sync::OnceCell;
use std::fmt::Write;
use std::{
    alloc::GlobalAlloc,
    sync::atomic::{AtomicBool, AtomicIsize, Ordering},
};

use dashmap::DashMap;

/// Entrypoint to use with [`GlobalAlloc`].
pub struct UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    alloc: A,
    frame_counter: OnceCell<DashMap<Vec<usize, std::alloc::System>, AtomicIsize>>,
    disabled: AtomicBool,

    sample_rate: usize,
    skip_stack: usize,
    fetch_stack: usize,
    capture_threshold: usize,
}

impl<A> UnkaiGlobalAlloc<A>
where
    A: GlobalAlloc,
{
    pub const fn new(
        alloc: A,
        sample_rate: usize,
        skip_stack: usize,
        fetch_stack: usize,
        capture_threshold: usize,
    ) -> Self {
        let frame_counter = OnceCell::<DashMap<Vec<usize, std::alloc::System>, AtomicIsize>>::new();
        Self {
            alloc,
            frame_counter,
            disabled: AtomicBool::new(false),

            sample_rate,
            skip_stack,
            fetch_stack,
            capture_threshold,
        }
    }

    pub fn report_addr(&self) -> Vec<(Vec<usize, std::alloc::System>, isize)> {
        let _ = backtrace::Backtrace::new();

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

        if layout.size() <= self.capture_threshold {
            return ptr;
        }

        let disabled = self
            .disabled
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err();

        if !disabled {
            if sampling(ptr, self.sample_rate) {
                let frames = partial_trace(self.skip_stack, self.fetch_stack);
                self.frame_counter
                    .get_or_init(DashMap::new)
                    .entry(frames)
                    .or_default()
                    .fetch_add(layout.size() as isize, Ordering::Relaxed);
            }
            self.disabled.store(false, Ordering::Relaxed);
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        if layout.size() <= self.capture_threshold {
            return;
        }

        if sampling(ptr, self.sample_rate) {
            let frames = partial_trace(self.skip_stack, self.fetch_stack);

            self.frame_counter
                .get_or_init(DashMap::new)
                .get(&frames)
                .map(|counter| counter.fetch_sub(layout.size() as isize, Ordering::Relaxed));
        }

        self.alloc.dealloc(ptr, layout)
    }
}

fn partial_trace(skip: usize, fetch: usize) -> Vec<usize, std::alloc::System> {
    let mut skipped = 0;
    let mut fetched = 0;
    let mut res = Vec::with_capacity_in(fetch, std::alloc::System);

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

fn sampling(ptr: *mut u8, rate: usize) -> bool {
    ((ptr as usize) >> 3) % rate == 0
}
