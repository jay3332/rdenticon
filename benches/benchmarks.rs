use criterion::{criterion_group, criterion_main, Criterion};

use rand::RngCore;
use rdenticon::{render_identicon, Config};

const SAMPLES: [u32; 10] = [16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192];

fn get_r_bytes<const SIZE: usize>() -> [u8; SIZE] {
    let mut r_bytes = [0; SIZE];
    rand::thread_rng().fill_bytes(&mut r_bytes);

    r_bytes
}

fn gen_const_size_no_save_var_bytes(c: &mut Criterion) {
    c.bench_function("size: 256, no save, bytes: 20, var", |b| {
        b.iter_batched(
            || (get_r_bytes(), Config::default()),
            |(hash, config)| render_identicon(hash, &config),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn gen_var_size_no_save_var_bytes(c: &mut Criterion) {
    for size in SAMPLES {
        c.bench_function(&format!("size: {size}, save: no, bytes: 20, var"), |b| {
            b.iter_batched(
                || (get_r_bytes(), Config::builder().size(size).build().unwrap()),
                |(hash, config)| render_identicon(hash, &config),
                criterion::BatchSize::SmallInput,
            )
        });
    }
}

criterion_group!(
    no_save,
    gen_const_size_no_save_var_bytes,
    gen_var_size_no_save_var_bytes
);
criterion_main!(no_save);
