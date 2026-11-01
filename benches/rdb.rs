//! Benchmarks for loading RDB snapshot files into the in-memory database.

use std::io::Cursor;

use divan::{black_box, Bencher};
use ipfsredis::rdb::load_from_reader;
use ipfsredis::storage::{Config, Db};

fn main() {
    divan::main();
}

const SIZES: &[usize] = &[10, 100, 1000];

/// Real RDB file produced by Redis 7.2.5, shipped in `examples/`.
const EXAMPLE_RDB: &[u8] = include_bytes!("../examples/dump.rdb");

fn push_string(out: &mut Vec<u8>, s: &str) {
    // Keep strings under 64 bytes so they use the 6-bit length encoding.
    assert!(s.len() < 64);
    out.push(u8::try_from(s.len()).unwrap());
    out.extend_from_slice(s.as_bytes());
}

/// Build a synthetic RDB file with `n` string keys.
/// When `with_expiry` is set, every key carries a millisecond expiration.
fn synthetic_rdb(n: usize, with_expiry: bool) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"REDIS0011");
    // metadata
    out.push(0xFA);
    push_string(&mut out, "redis-ver");
    push_string(&mut out, "7.2.5");
    // database selector + resize db
    out.extend_from_slice(&[0xFE, 0x00, 0xFB]);
    let n_u32 = u32::try_from(n).unwrap();
    let expires = if with_expiry { n_u32 } else { 0 };
    for size in [n_u32, expires] {
        out.push(0x80);
        out.extend_from_slice(&size.to_be_bytes());
    }
    // far in the future so keys never expire
    let expiration_ms: u64 = 4_102_444_800_000;
    for i in 0..n {
        if with_expiry {
            out.push(0xFC);
            out.extend_from_slice(&expiration_ms.to_le_bytes());
        }
        out.push(0x00);
        push_string(&mut out, &format!("key:{i}"));
        push_string(&mut out, &format!("value-for-key-{i}"));
    }
    out.push(0xFF);
    // checksum (not verified)
    out.extend_from_slice(&[0u8; 8]);
    out
}

fn bench_load(bencher: Bencher, data: &[u8]) {
    bencher
        .with_inputs(|| Db::new(Config::default()))
        .bench_local_values(|db| {
            let mut reader = Cursor::new(black_box(data));
            load_from_reader(&mut reader, &db).unwrap();
            db
        });
}

#[divan::bench]
fn load_example_dump(bencher: Bencher) {
    bench_load(bencher, EXAMPLE_RDB);
}

#[divan::bench(args = SIZES)]
fn load_keys(bencher: Bencher, n: usize) {
    bench_load(bencher, &synthetic_rdb(n, false));
}

#[divan::bench(args = SIZES)]
fn load_keys_with_expiry(bencher: Bencher, n: usize) {
    bench_load(bencher, &synthetic_rdb(n, true));
}
