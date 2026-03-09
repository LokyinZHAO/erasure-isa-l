use std::ops::Deref;

use crate::{Error, gf};

pub struct GaloisFiledTable(Vec<u8>);

impl GaloisFiledTable {
    pub fn try_from_matrix(matrix: &[u8], rows: usize, cols: usize) -> Result<Self, Error> {
        if matrix.len() != rows * cols {
            return Err(Error::invalid_arguments(format!(
                "Invalid matrix size: length {}, expected {} x {} = {}",
                matrix.len(),
                rows,
                cols,
                rows * cols
            )));
        }
        let mut gf_table = vec![0_u8; cols * rows * 32];
        crate::ec::init_tables(
            cols.try_into().unwrap(),
            rows.try_into().unwrap(),
            matrix,
            &mut gf_table[..],
        );
        Ok(Self(gf_table))
    }

    pub fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for GaloisFiledTable {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl Deref for GaloisFiledTable {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for GaloisFiledTable {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Performs a dot product of multiple source slices with the Galois field table
/// and writes the result into the destination buffer.
///
/// That is, it computes as follows:
/// `dest[i] = sum(source[j][i] * coef[j])` for each `i` in `dest`
///
/// This function computes the dot product of multiple source slices against a Galois field table,
/// which is typically used in erasure coding encoding and decoding operations.
///
/// # Requirements
/// Each buffer must be at least 32 bytes long, and the source slices must all be of equal length.
///
/// # Arguments
/// * `table` - The Galois field table generated from `source.len()` coefficients, which must be of length `32 * source.len()`.
/// * `source` - A slice of source slices, each of which must be equal in length to the destination buffer.
/// * `dest` - The destination buffer where the result will be written.
///
/// # Errors
/// The following errors can occur:
/// * `Error::invalid_arguments` - If the destination buffer is smaller than 32 bytes.
/// * `Error::invalid_arguments` - If the source slices are not all equal in length to the destination buffer.
/// * `Error::invalid_arguments` - If the length of the Galois field table does not match `32 * source.len()`.
///
/// # Example
/// ```rust
/// use erasure_isa_l::galois::{GaloisFiledTable, dot_prod};
/// use erasure_isa_l::Error;
/// use erasure_isa_l::gf::mul;
/// // choose the second row of the matrix
/// let table = GaloisFiledTable::try_from_matrix(&[1, 2, 3, 4][0..2], 1, 2).unwrap();
/// let source = vec![
///     (0..32).map(|_| 0_u8).collect::<Vec<u8>>(),
///     (32..64).map(|_| 0_u8).collect::<Vec<u8>>(),
/// ];
/// let mut dest = vec![0_u8; 32];
/// dot_prod(&table, &source, &mut dest).unwrap();
/// let expected = source[0].iter().zip(source[1].iter()).map(|(a, b)| mul(*a, 3) ^ mul(*b, 4)).collect::<Vec<u8>>();
/// assert_eq!(dest, expected);
/// ```
///
/// # WARNING
/// This function has bugs in the underlying C implementation that when the hardware supports GFNI instructions.
pub fn dot_prod<T>(
    table: &GaloisFiledTable,
    source: impl AsRef<[T]>,
    dest: &mut [u8],
) -> Result<(), Error>
where
    T: AsRef<[u8]>,
{
    let len = dest.len();
    if len < 32 {
        return Err(Error::invalid_arguments(format!(
            "Destination buffer too small: length {}, expected at least 32",
            len
        )));
    }
    let source = source.as_ref();
    if source.iter().any(|s| s.as_ref().len() != len) {
        return Err(Error::invalid_arguments(
            "Source slices must be equal in length to destination buffer",
        ));
    }
    if table.len() != 32 * source.len() {
        return Err(Error::invalid_arguments(format!(
            "Table length mismatch: expected 32 * source number {}, got {}",
            32 * source.len(),
            table.len()
        )));
    }
    let src_ptrs = source
        .iter()
        .map(|s| s.as_ref().as_ptr())
        .collect::<Vec<_>>();
    gf::vect_dot_prod(
        len.try_into().unwrap(),
        source.len().try_into().unwrap(),
        &table,
        &src_ptrs,
        dest,
    );
    Ok(())
}

/// Performs a multiply operation on a single source slice with the Galois field table
/// and adds the result into the destination buffer.
///
/// That is, it computes as follows:
/// `dest[i] += source_i[i] * coef[index]` for each `i` in `dest`
///
/// This function computes the product of a single source slice against a Galois field table,
/// which is typically used in erasure coding updates.
///
/// # Requirements
/// The destination buffer must be at least 64 bytes long,
/// and the source slice must be of the same length as the destination buffer.
///
/// # Arguments
/// * `table` - The Galois field table generated from `source_num` coefficients, which must be of length `32 * source_num`.
/// * `source_num` - The number of source slices.
/// * `index` - The index of the source slice to be multiplied.
/// * `source_i` - The source slice to be multiplied.
/// * `dest` - The destination buffer where the result will be added.
///
/// # Errors
/// The following errors can occur:
/// * `Error::invalid_arguments` - If the destination buffer is smaller than 64 bytes
/// * `Error::invalid_arguments` - If the source slice length does not match the destination buffer length.
/// * `Error::invalid_arguments` - If the length of the Galois field table does not match `32 * source_num`.
/// * `Error::invalid_arguments` - If the `index` is out of bounds for the number of source slices.
pub fn mul_add(
    table: &GaloisFiledTable,
    source_num: usize,
    index: usize,
    source_i: &[u8],
    dest: &mut [u8],
) -> Result<(), Error> {
    let len = dest.len();
    if len < 64 {
        return Err(Error::invalid_arguments(format!(
            "Destination buffer too small: length {}, expected at least 64",
            len
        )));
    }
    if source_i.len() != len {
        return Err(Error::invalid_arguments(format!(
            "Source slice length mismatch: source {}, dest {}",
            source_i.len(),
            len,
        )));
    }
    if table.len() != 32 * source_num {
        return Err(Error::invalid_arguments(format!(
            "Table length mismatch: expected 32 * source number {}, got {}",
            32 * source_num,
            table.len()
        )));
    }
    if index >= source_num {
        return Err(Error::invalid_arguments(format!(
            "Index out of bounds: index {}, source number {}",
            index, source_num
        )));
    }

    crate::gf::vect_mad(
        len.try_into().unwrap(),
        source_num.try_into().unwrap(),
        index.try_into().unwrap(),
        &table,
        source_i,
        dest,
    );
    Ok(())
}
