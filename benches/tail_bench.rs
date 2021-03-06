/*
 * Copyright 2021 StoneDot (Hiroaki Goto)
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

extern crate regtail;

use criterion::{criterion_group, criterion_main, Criterion};
use nix::unistd::sync;
use procfs::sys::vm::{drop_caches, DropCache};
use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use regtail::tail::{from_file_to_sink, tail_from_reader};
use std::cmp::min;
use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

fn setup_bench(bench_directory: &str) -> PathBuf {
    let dir = PathBuf::from(format!("benchmarks/{}", bench_directory));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[allow(dead_code)]
fn write_random(file: &mut File, size: usize, seed: <XorShiftRng as SeedableRng>::Seed) {
    const BUF_SIZE: usize = 8 * 1024;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789\n";
    let mut fs = BufWriter::new(file);
    let mut buf = [0; BUF_SIZE];
    let mut rest_size = size;
    let mut rng = XorShiftRng::from_seed(seed);
    while rest_size > 0 {
        let size = min(BUF_SIZE, rest_size);
        rest_size -= size;
        for i in 0..size {
            let idx = rng.gen_range(0..CHARSET.len());
            buf[i] = CHARSET[idx];
        }
        fs.write(&buf[0..size])
            .expect("Failed to write random string");
    }
}

#[allow(dead_code)]
pub fn put_random_file(path: &PathBuf, size: usize, seed: <XorShiftRng as SeedableRng>::Seed) {
    let file_path_str = path.to_str().unwrap();
    let mut fh = File::create(path).expect(format!("Failed to pen '{}", file_path_str).as_ref());
    write_random(&mut fh, size, seed);
    fh.sync_all()
        .expect(format!("Failed sync of file '{}'", file_path_str).as_ref());
}

#[cfg(target_os = "linux")]
fn big_file_tail(path: &PathBuf, lines: u64) {
    // Clear file caches
    sync();
    drop_caches(DropCache::All).expect("Failed to drop cache");

    // Start actual benchmark
    let mut state = from_file_to_sink(path).unwrap();
    tail_from_reader(&mut state, lines).unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    const LINES: u64 = 10000;
    let parent_path = setup_bench("big_file");
    let mut path = parent_path.clone();
    path.push("file");

    // Create 8MB file
    let seed = [
        82u8, 45, 71, 2, 135, 83, 121, 11, 44, 188, 87, 121, 96, 241, 192, 224,
    ];
    put_random_file(&path, 8 * 1024 * 1024, seed);

    c.bench_function("big_file_tail", |b| b.iter(|| big_file_tail(&path, LINES)));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
