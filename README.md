# unkai

Allocator proxy with statistics in "stack", "size" and "duration" axis.

## Supported Feature
- `GlobalAlloc`
    - Capture and record backtrace with memory consumption

## Todos
- `GlobalAlloc`
    - [ ] Record pointer's lifetime
- `Allocator`
    - [ ] Tree-structured
    - [ ] Low-overhead in-use statistics
- General
    - [ ] Prometheus integration
    - [ ] Build-in report generation

## Example

```bash
cargo run --example collections --release
```

```rust
#[global_allocator]
static UNKAI: UnkaiGlobalAlloc<Jemalloc> = UnkaiGlobalAlloc::new(Jemalloc {}, 99, 5, 10);

fn main() {
    let mut container = Vec::with_capacity(10000);
    for _ in 0..10000 {
        let item = Vec::<usize>::with_capacity(1000);
        container.push(item);
    }

    for (bt, count) in UNKAI.report_symbol() {
        println!("{} bytes are allocated from:", count);
        println!("{}", bt);
    }
}
```
