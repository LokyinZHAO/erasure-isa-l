use std::num::NonZeroUsize;

use erasure_isa_l::erasure::ErasureCode;

const BLOCK_LEN: usize = 512;
const K: usize = 4;
const M: usize = 2;

#[test]
fn invert_matrix() {
    // Inverse of identity matrix is identity matrix
    #[rustfmt::skip]
        let input: Vec<u8> = vec![
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x01,
        ];

    let mut output = vec![0_u8; input.len()];
    let actual = erasure_isa_l::gf::invert_matrix(
        input.clone().as_mut_slice(),
        output.as_mut_slice(),
        input.len().isqrt().try_into().unwrap(),
    );
    assert!(actual);
    let expected: Vec<u8> = input;
    assert_eq!(output, expected);

    // Cauchy bottom part
    #[rustfmt::skip]
        let input: Vec<u8> = vec![
            0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x01,
            0x47, 0xA7, 0x7A, 0xBA,
            0xA7, 0x47, 0xBA, 0x7A,
        ];
    let mut output = vec![0_u8; input.len()];
    let actual = erasure_isa_l::gf::invert_matrix(
        input.clone().as_mut_slice(),
        output.as_mut_slice(),
        input.len().isqrt().try_into().unwrap(),
    );
    assert!(actual);
    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0xD0, 0x6B, 0x44, 0x50,
        0x6B, 0xD0, 0x50, 0x44,
        0x01, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00,
    ];
    assert_eq!(output, expected);
}

#[test]
fn basic_test() {
    use erasure_isa_l::erasure::ErasureCode;

    let k = NonZeroUsize::new(K).unwrap();
    let m = NonZeroUsize::new(M).unwrap();
    let code = ErasureCode::with_reed_solomon(k, m).unwrap();

    // Further tests can be added here to validate encoding and decoding
    // Prepare input data: k blocks of BLOCK_LEN bytes
    let data: Vec<Vec<u8>> = (0..k.get()).map(|i| vec![i as u8; BLOCK_LEN]).collect();

    // Prepare parity blocks (m blocks of BLOCK_LEN bytes, initially zeroed)
    let mut parity: Vec<Vec<u8>> = vec![vec![0u8; BLOCK_LEN]; m.get()];

    // Encode: fill parity blocks
    code.encode(&data, &mut parity).expect("Encoding failed");

    // Erase some blocks
    let erausure = [2, 5];
    let mut erased_data: Vec<Vec<u8>> = data.clone();
    erased_data.get_mut(2).unwrap().fill(0);
    let mut erased_parity: Vec<Vec<u8>> = parity.clone();
    erased_parity.get_mut(1).unwrap().fill(0);

    // Decode
    code.decode(&mut erased_data, &mut erased_parity, erausure.to_vec())
        .expect("Decoding failed");

    // Check that recovered data matches original
    assert_eq!(erased_parity, parity);
    assert_eq!(erased_data, data);
}

#[test]
fn general_test_ec() {
    use erasure_isa_l::erasure::ErasureCode;
    let k_m_pair: [(usize, usize); 5] = [(4, 2), (6, 2), (9, 3), (10, 4), (16, 4)];
    for (k, m) in k_m_pair {
        let k = NonZeroUsize::new(k).unwrap();
        let m = NonZeroUsize::new(m).unwrap();
        let ec = ErasureCode::with_reed_solomon(k, m).unwrap();
        general_test(ec).expect("General test failed for Reed-Solomon code");
    }

    for (k, m) in k_m_pair {
        let k = NonZeroUsize::new(k).unwrap();
        let m = NonZeroUsize::new(m).unwrap();
        let ec = ErasureCode::with_cauchy(k, m).unwrap();
        general_test(ec).expect("General test failed for Cauchy code");
    }
}

#[test]
fn decode_table() {
    use erasure_isa_l::erasure::ErasureCode;
    let k = NonZeroUsize::new(K).unwrap();
    let m = NonZeroUsize::new(M).unwrap();
    let ec = ErasureCode::with_reed_solomon(k, m).unwrap();

    let orig_data = make_rand_blk(K, BLOCK_LEN);
    let mut orig_parity = make_zero_blk(M, BLOCK_LEN);
    ec.encode(&orig_data, &mut orig_parity)
        .expect("Encoding failed");
    let orig_parity = orig_parity;

    let erasures_list = [vec![2], vec![5], vec![0, 4]];
    for erasures in &erasures_list {
        let mut data = orig_data.clone();
        let mut parity = orig_parity.clone();
        // Set erased blocks to zero
        for &e in erasures {
            if e < K {
                data[e] = vec![0_u8; BLOCK_LEN];
            } else {
                parity[e - K] = vec![0_u8; BLOCK_LEN];
            }
        }
        let table = ec
            .make_decode_table(erasures.to_vec())
            .expect("Failed to get decode table");
        ec.decode_with_table(&mut data, &mut parity, &table, erasures.to_vec())
            .expect("Decoding with table failed");

        // Check that recovered data matches original
        assert_eq!(data, orig_data);
        assert_eq!(parity, orig_parity);
    }
}

#[test]
fn fail_test() {
    use erasure_isa_l::erasure::ErasureCode;
    let k = NonZeroUsize::new(K).unwrap();
    let m = NonZeroUsize::new(M).unwrap();
    let ec = ErasureCode::with_cauchy(k, m).unwrap();

    // Prepare input data: k blocks of BLOCK_LEN bytes
    let data: Vec<Vec<u8>> = make_rand_blk(K, BLOCK_LEN);
    // Prepare parity blocks (m blocks of BLOCK_LEN bytes, initially zeroed)
    let mut parity: Vec<Vec<u8>> = make_zero_blk(M, BLOCK_LEN);

    // Encode
    {
        let res = ec.encode(&data[0..K - 1], &mut parity);
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }
    {
        let res = ec.encode(&data, &mut parity[0..M - 1]);
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }
    {
        let mut data_not_same_len = data.clone();
        data_not_same_len.first_mut().unwrap().clear();
        let res = ec.encode(&data_not_same_len, &mut parity[0..M - 1]);
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }

    // Decode
    {
        // too much erasures
        let mut erased_data = data.clone();
        let mut erased_parity = parity.clone();
        let erasures = vec![0, 1, 2];
        let res = ec.decode(&mut erased_data, &mut erased_parity, erasures.to_vec());
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::TooManyErasures(..))
        ));
    }
    {
        // buffer sizes mismatch
        let mut erased_data = data.clone();
        let mut erased_parity = parity.clone();
        let erasures = vec![0, 1];
        erased_data[0].clear(); // Make data block empty
        let res = ec.decode(&mut erased_data, &mut erased_parity, erasures.to_vec());
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }
    {
        // erasures out of bounds
        let mut erased_data = data.clone();
        let mut erased_parity = parity.clone();
        let erasures = vec![10]; // 10 is out of bounds
        let res = ec.decode(&mut erased_data, &mut erased_parity, erasures.to_vec());
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }
    {
        // not enough data blocks
        let mut erased_data = data.clone();
        let mut erased_parity = parity.clone();
        let erasures = vec![0];
        let res = ec.decode(
            &mut erased_data[0..K - 1],
            &mut erased_parity,
            erasures.to_vec(),
        );
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
        let res = ec.decode(
            &mut erased_data,
            &mut erased_parity[0..M - 1],
            erasures.to_vec(),
        );
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }

    // DecodeTable
    {
        // too many erasures
        let erasures = vec![0, 1, 2];
        let res = ec.make_decode_table(erasures.to_vec());
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::TooManyErasures(..))
        ));
    }
    {
        // erasures out of bounds
        let erasures = vec![10]; // 10 is out of bounds
        let res = ec.make_decode_table(erasures.to_vec());
        assert!(matches!(
            res,
            Err(erasure_isa_l::erasure::Error::InvalidArguments(..))
        ));
    }
}

fn make_rand_blk(n: usize, blk_size: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|_| rand::random_iter().take(blk_size).collect::<Vec<u8>>())
        .collect()
}

fn make_zero_blk(n: usize, blk_size: usize) -> Vec<Vec<u8>> {
    (0..n).map(|_| vec![0_u8; blk_size]).collect()
}

fn general_test(ec: ErasureCode) -> Result<(), erasure_isa_l::erasure::Error> {
    let k = ec.source_num();
    let m = ec.code_num();
    let n = ec.block_num();

    let blk_size = BLOCK_LEN;

    let orig_data = make_rand_blk(k, blk_size);
    let mut orig_parity = make_zero_blk(m, blk_size);
    ec.encode(&orig_data, &mut orig_parity)?;
    let orig_parity = orig_parity;

    // no erasures
    let mut data = orig_data.clone();
    let mut parity = orig_parity.clone();
    ec.decode(&mut data, &mut parity, vec![])?;
    assert_eq!(data, orig_data);
    assert_eq!(parity, orig_parity);

    // randomly erase data block
    let erasure = rand::random_range(0..k);
    let mut erased_data = {
        let mut data = orig_data.clone();
        data[erasure] = vec![0_u8; blk_size];
        data
    };
    let mut parity = orig_parity.clone();
    ec.decode(&mut erased_data, &mut parity, vec![erasure])?;
    assert_eq!(erased_data, orig_data);
    assert_eq!(parity, orig_parity);

    // randomly erase parity block
    let erasure = rand::random_range(k..n);
    let mut erased_parity = {
        let mut parity = orig_parity.clone();
        parity[erasure - k] = vec![0_u8; blk_size];
        parity
    };
    let mut data = orig_data.clone();
    ec.decode(&mut data, &mut erased_parity, vec![erasure])?;
    assert_eq!(data, orig_data);
    assert_eq!(erased_parity, orig_parity);

    // randomly erase m data and parity blocks
    let mut erasures = std::collections::HashSet::new();
    while erasures.len() < m {
        erasures.insert(rand::random_range(0..n));
    }
    let mut erased_stripe = {
        let mut blks = orig_data
            .iter()
            .chain(orig_parity.iter())
            .cloned()
            .collect::<Vec<_>>();
        blks.iter_mut().enumerate().for_each(|(i, block)| {
            if erasures.contains(&i) {
                *block = vec![0_u8; blk_size];
            }
        });
        blks
    };
    let mut erasures = erasures.into_iter().collect::<Vec<_>>();
    erasures.sort_unstable();
    let (mut erased_data, mut erased_parity) = erased_stripe.split_at_mut(k);
    ec.decode(&mut erased_data, &mut erased_parity, erasures.to_vec())?;
    assert_eq!(erased_data, orig_data);
    assert_eq!(erased_parity, orig_parity);
    Ok(())
}
