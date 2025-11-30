# Performance Benchmarks

This directory contains performance benchmarks for NeuraDock.

## Running Benchmarks

### With Cargo (built-in)
```bash
cd apps/desktop/src-tauri
cargo test --release --test performance_benchmarks
```

### Usage Examples

The benchmarks measure:
- Database query performance (with and without indexes)
- Encryption/decryption throughput
- Check-in operation latency
- Aggregate reconstruction time

## Benchmark Results

Results will be printed to stdout with timing information.

## Adding New Benchmarks

Create new benchmark functions in `tests/performance_benchmarks.rs` following the existing patterns.
