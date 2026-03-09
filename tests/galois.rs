/// This test is disabled temporarily due to a bug in the underlying C implementation.
#[test]
#[allow(dead_code)]
fn dot_prod() {
    use erasure_isa_l::galois::{GaloisFiledTable, dot_prod};
    use erasure_isa_l::gf::mul;
    // choose the second row of the matrix
    const LEN: usize = 32;
    let table = GaloisFiledTable::try_from_matrix(&[1_u8, 2, 3, 4][2..], 1, 2).unwrap();
    let source = vec![
        (0..).take(LEN).collect::<Vec<u8>>(),
        (32..).take(LEN).collect::<Vec<u8>>(),
    ];
    let mut dest = vec![0_u8; 32];
    dot_prod(&table, &source, &mut dest).unwrap();
    let expected = source[0]
        .iter()
        .zip(source[1].iter())
        .map(|(a, b)| mul(*a, 3) ^ mul(*b, 4))
        .collect::<Vec<u8>>();
    assert_eq!(dest, expected);
}

#[test]
fn mad() {
    use erasure_isa_l::galois::{GaloisFiledTable, mul_add};
    // choose the second row of the matrix
    const LEN: usize = 64;
    let table = GaloisFiledTable::try_from_matrix(&[1_u8, 2, 3, 4][2..], 1, 2).unwrap();
    let source = vec![
        (0..).take(LEN).collect::<Vec<u8>>(),
        (32..).take(LEN).collect::<Vec<u8>>(),
    ];
    let mut dest = vec![0_u8; LEN];
    erasure_isa_l::galois::dot_prod(&table, &source, &mut dest).unwrap();
    // reverse the second source
    let mut source_update = source.clone();
    source_update[1].reverse();
    let source_update = source_update;
    // multiply the first source by 2 and add to the destination
    let delta = source_update[1]
        .iter()
        .zip(&source[1])
        .map(|(a, b)| *a ^ *b)
        .collect::<Vec<_>>();
    mul_add(&table, 2, 1, &delta, &mut dest).unwrap();
    let mut expected = vec![0_u8; LEN];
    erasure_isa_l::galois::dot_prod(&table, &source_update, &mut expected).unwrap();
    assert_eq!(dest, expected);
}
