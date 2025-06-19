use std::num::NonZeroUsize;

const BLOCK_LEN: usize = 32;
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
fn general_test() {
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
    code.decode(&mut erased_data, &mut erased_parity, &erausure)
        .expect("Decoding failed");

    // Check that recovered data matches original
    assert_eq!(erased_parity, parity);
    assert_eq!(erased_data, data);
}
