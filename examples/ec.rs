use std::num::NonZeroUsize;

use erasure_isa_l::erasure::ErasureCode;

fn usage(cmd: &str) {
    let cmd = cmd.split('/').last().unwrap_or(cmd);
    println!("Usage: {cmd} <source_num> <code_num> <block size(MB)> <test_load>");
    println!("Example: {cmd} 10 4 1 1024");
}

fn make_rand_blk(n: usize, blk_size: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|_| rand::random_iter().take(blk_size).collect::<Vec<u8>>())
        .collect()
}

fn make_zero_blk(n: usize, blk_size: usize) -> Vec<Vec<u8>> {
    (0..n).map(|_| vec![0_u8; blk_size]).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        usage(&args[0]);
        return;
    }
    let k = args[1].parse::<usize>().expect("invalude source_num");
    let m = args[2].parse::<usize>().expect("invalude code_num");
    let block_size = {
        let mb = args[3].parse::<usize>().expect("invalude block size");
        mb * 1024 * 1024 // convert MB to bytes
    };

    let test_load = args[4].parse::<usize>().expect("invalude test_load");
    let ec = ErasureCode::with_cauchy(NonZeroUsize::new(k).unwrap(), NonZeroUsize::new(m).unwrap())
        .unwrap();
    let mut src = make_rand_blk(k, block_size);
    let mut code = make_zero_blk(m, block_size);
    let epoch = std::time::Instant::now();
    for _ in 0..test_load {
        // encode
        ec.encode(&src, &mut code).unwrap();
    }
    let encode_elapsed = epoch.elapsed();
    let epoch = std::time::Instant::now();
    let decode_tables = (0..ec.block_num())
        .map(|i| ec.make_decode_table(vec![i]).unwrap())
        .collect::<Vec<_>>();
    for i in 0..test_load {
        let erasure = i % ec.block_num();
        let decode_table = &decode_tables[erasure];
        ec.decode_with_table(&mut src, &mut code, decode_table, vec![erasure])
            .unwrap();
    }
    let decode_elapsed = epoch.elapsed();
    println!(
        "Encode elapsed: {encode_elapsed:?}, throughput: {:.2} MB/s",
        (k * block_size * test_load) as f64 / encode_elapsed.as_secs_f64() / 1024.0 / 1024.0
    );
    println!(
        "Decode elapsed: {decode_elapsed:?}, throughput: {:.2} MB/s",
        (k * block_size * test_load) as f64 / decode_elapsed.as_secs_f64() / 1024.0 / 1024.0
    );
}
