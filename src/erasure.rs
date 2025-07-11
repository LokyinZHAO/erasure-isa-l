//! This module provides high-level abstractions for erasure coding using the `isa-l` library.
//!
//! It allows users to encode and decode data with erasure codes, handling the complexities of the underlying `isa-l` library.
//! And it do more checks to ensure the input data is valid.
use std::num::NonZeroUsize;

use crate::{ec, gf};

/// The `Error` enum defines the possible errors that this crate can occur.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// TooManyErasure: The number of erasures is larger than the maximum allowed,
    /// and the lost data cannot be recovered.
    #[error("Too Many Erased Blocks: {0} erased, up to {1} allowed")]
    TooManyErasures(usize, usize),
    /// InvalidArguments: The the input is invalid.
    #[error("Invalid Arguments: {0}")]
    InvalidArguments(String),
    /// InternalError: An internal error caused by libisa-l.
    #[error("Internal Error: {0}")]
    InternalError(String),
    /// Other: Other errors that are not covered by the above.
    #[error("Error: {0}")]
    Other(String),
}

#[allow(dead_code)]
impl Error {
    fn too_many_erasures(erasures: usize, max_erasures: usize) -> Self {
        Self::TooManyErasures(erasures, max_erasures)
    }

    fn invalid_arguments(msg: impl Into<String>) -> Self {
        Self::InvalidArguments(msg.into())
    }

    fn internal_error(msg: impl Into<String>) -> Self {
        Self::InternalError(msg.into())
    }

    fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

fn make_table_from_matrix(matrix: &[u8], cols: usize, rows: usize) -> Result<Vec<u8>, Error> {
    if matrix.len() != cols * rows {
        return Err(Error::invalid_arguments(format!(
            "matrix length {} is not equal to col * row {}",
            matrix.len(),
            cols * rows
        )));
    }
    let mut gf_table = vec![0_u8; cols * rows * 32];
    ec::init_tables(
        cols.try_into().unwrap(),
        rows.try_into().unwrap(),
        matrix,
        &mut gf_table[..],
    );
    Ok(gf_table)
}

/// ErasureCode is a struct that implements erasure coding by isa-l.
///
/// It provides some handy methods to encode and decode data ontop of the isa-l,
/// and do not require the user to deal with the low-level details of the isa-l library.
///
/// It do some checks to ensure the input data is valid, and will return an error if the input is invalid.
pub struct ErasureCode {
    k: i32,
    m: i32,
    encode_matrix: Vec<u8>,
    encode_gf_table: Vec<u8>,
}

enum CodeType {
    ReedSolomon,
    Cauchy,
}

/// DecodeTable is a struct that contains the decode table for acclerating coding.
///
/// It is generated by the [`ErasureCode::make_decode_table`].
pub struct DecodeTable(Vec<u8>);

impl ErasureCode {
    /// Creates a new `ErasureCode` instance with cauchy matrix.
    ///
    /// # Arguments
    /// * `source_num` - The number of source data blocks.
    /// * `code_num` - The number of code blocks.
    ///
    /// # Note
    /// Any sub matrix from a cauchy matrix is always invertable, and it is suitable for large pairs
    /// of `source_num` and `code_num`.
    pub fn with_cauchy(source_num: NonZeroUsize, code_num: NonZeroUsize) -> Result<Self, Error> {
        Self::new(
            source_num.get().try_into().unwrap(),
            code_num.get().try_into().unwrap(),
            CodeType::Cauchy,
        )
    }

    /// Creates a new `ErasureCode` instance with reed solomon matrix.
    ///
    /// # Arguments
    /// * `source_num` - The number of source data blocks.
    /// * `code_num` - The number of code blocks.  
    ///
    /// # Note
    /// For large pairs of `source_num` and `code_num`, it is possible to find
    /// cases where the decode matrix chosen from sources and parity is not invertable.
    /// You may want to adjust for certain pairs `source_num` and `code_num`.
    /// If the pair satisfies one of the following inequalities,
    /// no adjustment is required:
    /// * `source_num` <= 3
    /// * `source_num` = 4, `code_num` <= 21
    /// * `source_num` = 5, `code_num` <= 10
    /// * `source_num`= 21, `code_num` =4
    /// * `code_num` <= 3
    ///
    /// If your pair does not satisfy the above inequalities, and you don't want to adjust it,
    /// you can use the [`with_cauchy`](Self::with_cauchy) method instead, which is always invertable.
    pub fn with_reed_solomon(
        source_num: NonZeroUsize,
        code_num: NonZeroUsize,
    ) -> Result<Self, Error> {
        Self::new(
            source_num.get().try_into().unwrap(),
            code_num.get().try_into().unwrap(),
            CodeType::ReedSolomon,
        )
    }

    /// Returns the number of source data blocks.
    pub fn source_num(&self) -> usize {
        self.k as usize
    }

    /// Returns the number of code blocks.
    pub fn code_num(&self) -> usize {
        self.m as usize
    }

    /// Returns the total number of blocks (source + code).
    pub fn block_num(&self) -> usize {
        (self.k + self.m) as usize
    }

    /// Encodes the source data into code blocks.
    ///
    /// # Arguments
    /// * `data` - The source data blocks to be encoded.
    /// * `code` - The code blocks to be filled with the encoded data.
    ///
    /// # Errors
    /// The following errors can occur:
    /// * `Error::InvalidArguments` - If the data blocks number is not equal to the source number,
    ///   or the code blocks number is not equal to the code number.
    /// * `Error::InvalidArguments` - If the input data or code blocks do not have the same length
    ///
    /// # Examples
    /// ```rust
    /// # use erasure_isa_l::erasure::ErasureCode;
    /// # use std::num::NonZeroUsize;
    /// const BLOCK_LEN: usize = 1024;
    /// let k = NonZeroUsize::new(4).unwrap();
    /// let m = NonZeroUsize::new(2).unwrap();
    /// let ec = ErasureCode::with_reed_solomon(k, m).unwrap();
    /// let data: Vec<Vec<u8>> = (0..k.get()).map(|i| vec![i as u8; BLOCK_LEN]).collect();
    /// let mut parity: Vec<Vec<u8>> = vec![vec![0u8; BLOCK_LEN]; m.get()];
    /// ec.encode(&data, &mut parity).expect("Encoding failed");
    /// ```
    pub fn encode<T: AsRef<[u8]>, U: AsMut<[u8]>>(
        &self,
        data: impl AsRef<[T]>,
        mut code: impl AsMut<[U]>,
    ) -> Result<(), Error> {
        self.check_encode_buffer(&data, &mut code)?;
        self.encode_impl(data, code)
    }

    /// Encodes the source data into code blocks and returns the code blocks as `Vec<Vec<u8>>`.
    ///
    /// This is a convenience method that allocates a new `Vec<Vec<u8>>` for the code blocks,
    /// and encodes the data into it.
    ///
    /// See [`encode`](Self::encode) for more details on encoding.
    pub fn encode_to_owned<T: AsRef<[u8]>>(
        &self,
        data: impl AsRef<[T]>,
    ) -> Result<Vec<Vec<u8>>, Error> {
        let len = data.as_ref().first().unwrap().as_ref().len();
        let mut code = vec![vec![0_u8; len]; self.code_num()];
        self.encode(data, &mut code)?;
        Ok(code)
    }

    /// Update parities from a delta of a single source data block.
    ///
    /// This method is used to update the parity data from a single source data block
    /// without re-encoding all the data blocks.
    ///
    /// This method can also be used with a part of updated source block, see in the example.
    ///
    /// # Arguments
    /// * `index` - The index of the updated source data block.
    /// * `delta` - The delta of the updated source data block, which is the xor of the old and new data.
    /// * `code` - The code blocks to be updated with the new parity data.
    ///
    /// # Errors
    /// The following errors can occur:
    /// * `Error::InvalidArguments` - If the index is out of range `0..source_num()`.
    /// * `Error::InvalidArguments` - The code blocks number is not equal to the code number,
    /// * `Error::InvalidArguments` - If the input data or code blocks do not have the same length.
    ///
    /// # Examples
    /// ```rust
    /// # use erasure_isa_l::erasure::ErasureCode;
    /// # use std::num::NonZeroUsize;
    /// const BLOCK_LEN: usize = 1024;
    /// let k = NonZeroUsize::new(4).unwrap();
    /// let m = NonZeroUsize::new(2).unwrap();
    /// let ec = ErasureCode::with_reed_solomon(k, m).unwrap();
    /// let mut data: Vec<Vec<u8>> = (0..k.get()).map(|i| vec![i as u8; BLOCK_LEN]).collect();
    /// let mut parity: Vec<Vec<u8>> = vec![vec![0u8; BLOCK_LEN]; m.get()];
    /// ec.encode(&data, &mut parity).expect("Encoding failed");
    /// // Update the second quarter of the first source data block
    /// let update_index = 0;
    /// let range = BLOCK_LEN / 4.. BLOCK_LEN / 2;
    /// let delta = vec![0xCC_u8; range.len()];
    /// let parity_slice = parity.iter_mut().map(|p| &mut p[range.clone()]).collect::<Vec<_>>();
    /// ec.update(update_index, delta.as_slice(), parity_slice).expect("Update failed");
    /// // The parity blocks are updated correctly
    /// data[0][range.clone()].iter_mut().for_each(|x| *x ^= 0xCC);
    /// let mut expected_parity: Vec<Vec<u8>> = vec![vec![0u8; BLOCK_LEN]; m.get()];
    /// ec.encode(&data, &mut expected_parity).expect("Re-encoding failed");
    /// assert_eq!(parity, expected_parity);
    /// ```
    pub fn update<U: AsMut<[u8]>>(
        &self,
        index: usize,
        delta: &[u8],
        mut code: impl AsMut<[U]>,
    ) -> Result<(), Error> {
        self.check_update(index, delta, &mut code)?;
        // Update the data block at the given index
        ec::encode_data_update(
            delta.len().try_into().unwrap(),
            self.k_i32(),
            self.m_i32(),
            index.try_into().unwrap(),
            &self.encode_gf_table,
            delta,
            code.as_mut()
                .iter_mut()
                .map(AsMut::as_mut)
                .map(<[u8]>::as_mut_ptr)
                .collect::<Vec<_>>()
                .as_mut_slice(),
        );
        Ok(())
    }

    /// Decode the erased blocks from the surviving data and code blocks.
    ///
    /// The range of the blocks is `0..block_num()`.
    /// The first `source_num()` blocks are the source data blocks, the next `code_num()` blocks are the code blocks.
    ///
    /// The content of the erased blocks will be recovered and written back to the `data` and `code` buffers.
    /// And the content of the non-erased blocks will not be changed.
    ///
    /// # Arguments
    /// * `data` - The data blocks.
    /// * `code` - The code blocks.
    /// * `erasures` - The indices of the erased blocks.
    ///
    /// # Errors
    /// The following errors can occur:
    /// * `Error::TooManyErasure` - If the number of erasures is larger than the code number.
    /// * `Error::InvalidArguments` - If the erasure indices are out of range.
    /// * `Error::InvalidArguments` - If the data blocks number is not equal to the source number,
    ///   or the code blocks number is not equal to the code number.
    /// * `Error::InvalidArguments` - If the input data or code blocks do not have the same length
    /// * `Error::InternalError` - If the internal error occurs while decoding, typically due to a failure when
    ///   inverting the matrix.
    ///   
    /// # Note
    /// The order of the indices in `erasures` does not matter, and the duplicated indices will
    /// be treated as one erasure of the same block. So it is possible to pass a vector with
    /// more than the code number of erasures.
    ///
    /// A `DecodeTable` will be generated internally to perform the decoding, which is time consuming.
    /// If you need to decode multiple times with the same erasures, you can use [`make_decode_table`](Self::make_decode_table) to generate a
    /// `DecodeTable` and pass it to [`decode_with_table`](Self::decode_with_table) to avoid the overhead.
    ///
    /// # Examples
    /// ```rust
    /// # use erasure_isa_l::erasure::ErasureCode;
    /// # use std::num::NonZeroUsize;
    /// const BLOCK_LEN: usize = 1024;
    /// let k = NonZeroUsize::new(4).unwrap();
    /// let m = NonZeroUsize::new(2).unwrap();
    /// let code = ErasureCode::with_reed_solomon(k, m).unwrap();
    /// let ec = ErasureCode::with_reed_solomon(k, m).unwrap();
    /// let data: Vec<Vec<u8>> = (0..k.get()).map(|i| vec![i as u8; BLOCK_LEN]).collect();
    /// let mut parity: Vec<Vec<u8>> = vec![vec![0u8; BLOCK_LEN]; m.get()];
    /// ec.encode(&data, &mut parity).expect("Encoding failed");
    /// // Simulate erasures
    /// let mut erasures = vec![2, 5]; // Assume blocks 2 and 5 are erased
    /// let mut erased_data: Vec<Vec<u8>> = data.clone();
    /// erased_data[2] = vec![0; BLOCK_LEN];
    /// let mut erased_parity: Vec<Vec<u8>> = parity.clone();
    /// erased_parity[1] = vec![0; BLOCK_LEN];
    /// // Decode the erased blocks
    /// ec.decode(&mut erased_data, &mut erased_parity, erasures).expect("Decoding failed");
    /// // Verify that the erased blocks are recovered
    /// assert_eq!(&data, &erased_data);
    /// assert_eq!(&parity, &erased_parity);
    /// ```
    pub fn decode<U: AsMut<[u8]>>(
        &self,
        mut data: impl AsMut<[U]>,
        mut code: impl AsMut<[U]>,
        mut erasures: Vec<usize>,
    ) -> Result<(), Error> {
        self.check_decode_erasure(&mut erasures)?;
        self.check_decode_buffer(&mut data, &mut code)?;
        let decode_gf_table = self.make_decode_table_impl(erasures.as_slice())?;
        self.decode_impl(data, code, &decode_gf_table.0, erasures.as_slice())
    }

    /// Decode the erased blocks from the surviving data and code blocks using a pre-generated `DecodeTable`.
    ///
    /// The range of the blocks is `0..block_num()`.
    /// The first `source_num()` blocks are the source data blocks, the next `code_num()` blocks are the code blocks.
    ///
    /// The content of the erased blocks will be recovered and written back to the `data` and `code` buffers.
    /// And the content of the non-erased blocks will not be changed.
    ///
    /// A `DecodeTable` can be generated by [`make_decode_table`](Self::make_decode_table) method,
    /// and it can be reused for multiple decode operations with the same erasures.
    /// If you only need to decode once, you can use [`decode`](Self::decode) method instead,
    /// which will generate the `DecodeTable` internally.
    ///
    /// # Arguments
    /// * `data` - The data blocks.
    /// * `code` - The code blocks.
    /// * `decode_table` - The pre-generated `DecodeTable` for decoding.
    /// * `erasures` - The indices of the erased blocks.
    ///   
    /// # Errors
    /// The following errors can occur:
    /// * `Error::TooManyErasure` - If the number of erasures is larger than the code number.
    /// * `Error::InvalidArguments` - If the erasure indices are out of range.
    /// * `Error::InvalidArguments` - If the data blocks number is not equal to the source number,
    ///   or the code blocks number is not equal to the code number.
    /// * `Error::InvalidArguments` - If the input data or code blocks do not have the same length
    /// * `Error::InternalError` - If the internal error occurs while decoding, typically due to a failure when
    ///   inverting the matrix.
    ///   
    /// # Note
    /// The `decode_table` must be generated with the same erasures as the `erasures` argument.
    /// Otherwise, the decoding result is undefined.
    ///
    /// The order of the indices in `erasures` does not matter, and the duplicated indices will
    /// be treated as one erasure of the same block. So it is possible to pass a vector with
    /// more than the code number of erasures.
    ///
    /// # Examples
    /// ```rust
    /// # use erasure_isa_l::erasure::{ErasureCode, DecodeTable};
    /// # use std::num::NonZeroUsize;
    /// const BLOCK_LEN: usize = 1024;
    /// let k = NonZeroUsize::new(4).unwrap();
    /// let m = NonZeroUsize::new(2).unwrap();
    /// let ec = ErasureCode::with_reed_solomon(k, m).unwrap();
    /// let data: Vec<Vec<u8>> = (0..k.get()).map(|i| vec![i as u8; BLOCK_LEN]).collect();
    /// let mut parity: Vec<Vec<u8>> = vec![vec![0u8; BLOCK_LEN]; m.get()];
    /// ec.encode(&data, &mut parity).expect("Encoding failed");
    /// // Simulate erasures
    /// let mut erasures = vec![2, 5]; // Assume blocks 2 and 5 are erased
    /// let mut erased_data: Vec<Vec<u8>> = data.clone();
    /// erased_data[2] = vec![0; BLOCK_LEN];
    /// let mut erased_parity: Vec<Vec<u8>> = parity.clone();
    /// erased_parity[1] = vec![0; BLOCK_LEN];
    /// // Generate a decode table for the erasures
    /// let decode_table = ec.make_decode_table(erasures.clone()).expect("Failed to make decode table");
    /// // Decode the erased blocks using the decode table
    /// ec.decode_with_table(&mut erased_data, &mut erased_parity, &decode_table, erasures).expect("Decoding failed");
    /// // Verify that the erased blocks are recovered
    /// assert_eq!(&data, &erased_data);
    /// assert_eq!(&parity, &erased_parity);
    /// ```
    pub fn decode_with_table<U>(
        &self,
        mut data: impl AsMut<[U]>,
        mut code: impl AsMut<[U]>,
        decode_table: &DecodeTable,
        mut erasures: Vec<usize>,
    ) -> Result<(), Error>
    where
        U: AsMut<[u8]>,
    {
        self.check_decode_erasure(&mut erasures)?;
        self.check_decode_buffer(&mut data, &mut code)?;
        self.decode_impl(data, code, &decode_table.0, erasures.as_mut_slice())
    }

    /// Generates a `DecodeTable` for the given erasures.
    ///
    /// # Errors
    /// The following errors can occur:
    /// * `Error::TooManyErasure` - If the number of erasures is larger than the code number.
    /// * `Error::InvalidArguments` - If the erasure indices are out of range.
    ///
    /// # Note
    /// The order of the indices in `erasures` does not matter, but they must be unique.
    /// The duplicated indices will be treated as multiple erasures of the same block,
    /// and the result is undefined.
    pub fn make_decode_table(&self, mut erasures: Vec<usize>) -> Result<DecodeTable, Error> {
        self.check_decode_erasure(&mut erasures)?;
        self.make_decode_table_impl(erasures.as_mut_slice())
    }
}

/// private implementation of ErasureCode
impl ErasureCode {
    fn new(source_num: i32, code_num: i32, code_type: CodeType) -> Result<Self, Error> {
        let k: i32 = source_num;
        let m: i32 = code_num;
        let n = k + m;

        let mat_gen_fn = match code_type {
            CodeType::ReedSolomon => crate::gf::gen_rs_matrix,
            CodeType::Cauchy => crate::gf::gen_cauchy1_matrix,
        };
        let mut encode_matrix = vec![0; (k * n).try_into().unwrap()];
        mat_gen_fn(&mut encode_matrix, n, k);

        let mut gf_table = vec![0; (k * m * 32).try_into().unwrap()];
        ec::init_tables(
            k,
            m,
            &encode_matrix[usize::try_from(k * k).unwrap()..],
            &mut gf_table,
        );

        Ok(Self {
            k,
            m,
            encode_matrix,
            encode_gf_table: gf_table,
        })
    }

    fn k_i32(&self) -> i32 {
        self.k
    }

    fn m_i32(&self) -> i32 {
        self.m
    }

    #[allow(dead_code)]
    fn n_i32(&self) -> i32 {
        self.k + self.m
    }

    fn encode_impl<T: AsRef<[u8]>, U: AsMut<[u8]>>(
        &self,
        data: impl AsRef<[T]>,
        mut code: impl AsMut<[U]>,
    ) -> Result<(), Error> {
        let data_ptrs = data
            .as_ref()
            .iter()
            .map(AsRef::as_ref)
            .map(<[u8]>::as_ptr)
            .collect::<Vec<_>>();
        let mut code_ptrs = code
            .as_mut()
            .iter_mut()
            .map(AsMut::as_mut)
            .map(<[u8]>::as_mut_ptr)
            .collect::<Vec<_>>();
        let blk_len = data
            .as_ref()
            .first()
            .unwrap()
            .as_ref()
            .len()
            .try_into()
            .unwrap();
        ec::encode_data(
            blk_len,
            self.k_i32(),
            self.m_i32(),
            &self.encode_gf_table,
            &data_ptrs,
            &mut code_ptrs,
        );
        Ok(())
    }

    fn decode_impl<U: AsMut<[u8]>>(
        &self,
        mut data: impl AsMut<[U]>,
        mut code: impl AsMut<[U]>,
        decode_table: &[u8],
        erasures: &[usize],
    ) -> Result<(), Error> {
        let mut recover_src = Vec::with_capacity(self.block_num() - erasures.len());
        let mut recover_output = Vec::with_capacity(erasures.len());
        data.as_mut()
            .iter_mut()
            .chain(code.as_mut().iter_mut())
            .enumerate()
            .for_each(|(i, ptr)| {
                if erasures.contains(&i) {
                    // if the block is erased, we will recover it
                    recover_output.push(ptr.as_mut().as_mut_ptr());
                } else {
                    // if the block is not erased, we will use it to recover
                    recover_src.push(ptr.as_mut().as_ptr());
                }
            });
        let blk_len = data.as_mut().first_mut().unwrap().as_mut().len();
        ec::encode_data(
            blk_len.try_into().unwrap(),
            self.k,
            erasures.len().try_into().unwrap(),
            decode_table,
            &recover_src,
            &mut recover_output,
        );
        Ok(())
    }

    fn make_decode_table_impl(&self, erasures: &[usize]) -> Result<DecodeTable, Error> {
        let matrix = self.make_decode_matrix(erasures)?;
        let col = self.k as usize;
        let row = erasures.len();
        let table = make_table_from_matrix(&matrix[0..(col * row)], col, row)?;
        Ok(DecodeTable(table))
    }

    fn check_update<U: AsMut<[u8]>>(
        &self,
        index: usize,
        delta: &[u8],
        mut code: impl AsMut<[U]>,
    ) -> Result<(), Error> {
        if index >= self.source_num() {
            return Err(Error::invalid_arguments(format!(
                "index {} is out of range, max index is source number {}",
                index,
                self.source_num() - 1
            )));
        }

        if code.as_mut().len() != self.code_num() {
            return Err(Error::invalid_arguments(format!(
                "code length {} is not equal to code number {}",
                code.as_mut().len(),
                self.code_num()
            )));
        }

        let len = delta.len();
        for s in code.as_mut() {
            if s.as_mut().len() != len {
                return Err(Error::invalid_arguments(format!(
                    "code block length {} is not equal to delta length {}",
                    s.as_mut().len(),
                    len
                )));
            }
        }
        Ok(())
    }

    fn check_encode_buffer<T: AsRef<[u8]>, U: AsMut<[u8]>>(
        &self,
        data: impl AsRef<[T]>,
        mut code: impl AsMut<[U]>,
    ) -> Result<(), Error> {
        let data = data.as_ref();
        let code = code.as_mut();
        if data.len() != self.source_num() {
            return Err(Error::invalid_arguments(format!(
                "data length {} is not equal to source num {}",
                data.len(),
                self.k,
            )));
        }
        if code.len() != self.code_num() {
            return Err(Error::invalid_arguments(format!(
                "code length {} is not equal to code number {}",
                code.len(),
                self.m,
            )));
        }
        let len = data.first().unwrap().as_ref().len();
        for s in data.iter() {
            if s.as_ref().len() != len {
                return Err(Error::invalid_arguments("source data block must be equal"));
            }
        }
        for s in code.iter_mut() {
            if s.as_mut().len() != len {
                return Err(Error::invalid_arguments("code data block must be equal"));
            }
        }
        Ok(())
    }

    fn check_decode_buffer<U: AsMut<[u8]>>(
        &self,
        mut data: impl AsMut<[U]>,
        mut code: impl AsMut<[U]>,
    ) -> Result<(), Error> {
        let data = data.as_mut();
        let code = code.as_mut();
        if data.len() != self.k as usize {
            return Err(Error::invalid_arguments(format!(
                "data length {} is not equal to source num {}",
                data.len(),
                self.k,
            )));
        }
        if code.len() != self.m as usize {
            return Err(Error::invalid_arguments(format!(
                "code length {} is not equal to code number {}",
                code.len(),
                self.m,
            )));
        }
        let len = data.first_mut().unwrap().as_mut().len();
        for s in data.iter_mut() {
            if s.as_mut().len() != len {
                return Err(Error::invalid_arguments("source data block must be equal"));
            }
        }
        for s in code.iter_mut() {
            if s.as_mut().len() != len {
                return Err(Error::invalid_arguments("code data block must be equal"));
            }
        }
        Ok(())
    }

    fn check_decode_erasure(&self, erasures: &mut Vec<usize>) -> Result<(), Error> {
        erasures.sort_unstable();
        erasures.dedup();
        if erasures.len() > self.code_num() {
            return Err(Error::too_many_erasures(erasures.len(), self.code_num()));
        }
        if erasures.iter().any(|e| *e >= self.block_num()) {
            return Err(Error::invalid_arguments(format!(
                "erasure index out of range: {}",
                erasures
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        Ok(())
    }

    fn make_decode_matrix(&self, erasures: &[usize]) -> Result<Vec<u8>, Error> {
        let k = self.source_num();
        // record the erasure status for each block,
        // if the block is erased, set it to true
        let block_in_erasure = {
            let mut block_in_erasure = vec![false; self.block_num()];
            for &e in erasures.iter() {
                block_in_erasure[e] = true;
            }
            block_in_erasure
        };
        let decode_index = block_in_erasure
            .iter()
            .enumerate()
            // take the non-erased blocks
            .filter(|(_, e)| !**e)
            .take(k)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        let mut surviver_row = decode_index
            .iter()
            .flat_map(|&i| &self.encode_matrix[(k * i)..(k * i + k)])
            .copied()
            .collect::<Vec<_>>();
        debug_assert_eq!(surviver_row.len(), self.source_num() * self.source_num());

        // invert matrix to get recovered matrix
        let invert_matrix = {
            let mut invert_matrix = vec![0; k * k];
            let ret = gf::invert_matrix(&mut surviver_row, &mut invert_matrix, self.k_i32());
            if !ret {
                return Err(Error::internal_error("fail to invert matrix"));
            }
            invert_matrix
        };

        let mut decode_matrix = vec![0_u8; k * self.block_num()];
        for (i, erasure) in erasures.iter().enumerate() {
            let mat_row = &mut decode_matrix[(k * i)..(k * (i + 1))];
            match *erasure {
                erasure if erasure < k => {
                    // the erasure is in the source data
                    mat_row.copy_from_slice(
                        &invert_matrix[(k * erasure)..(k * erasure + self.source_num())],
                    );
                }
                erasure if erasure >= self.source_num() && erasure < self.block_num() => {
                    // the erasure is in the code data
                    mat_row.iter_mut().enumerate().for_each(|(i, s)| {
                        for j in 0..self.source_num() {
                            *s ^= gf::mul(
                                invert_matrix[j * k + i],
                                self.encode_matrix[k * erasure + j],
                            );
                        }
                    });
                }
                _ => unreachable!(),
            }
        }

        Ok(decode_matrix)
    }
}

#[cfg(test)]
mod test {
    use std::num::NonZeroUsize;

    const K: usize = 4;
    const M: usize = 2;

    #[test]
    fn make_decode_matrix() {
        let ec = super::ErasureCode::with_cauchy(
            NonZeroUsize::new(K).unwrap(),
            NonZeroUsize::new(M).unwrap(),
        )
        .unwrap();

        // One data and one parity error
        let decode_matrix = ec.make_decode_matrix(&[3, 4]).unwrap();
        #[rustfmt::skip]
        let expected_decode_matrix: Vec<u8> = vec![
            0xF5, 0x8F, 0xBB, 0x06,
            0x60, 0x40, 0xFE, 0xBB,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(decode_matrix, expected_decode_matrix);

        // one data error
        let decode_matrix = ec.make_decode_matrix(&[3]).unwrap();
        #[rustfmt::skip]
        let expected_decode_matrix: Vec<u8> = vec![
            0xC8, 0x52, 0x7B, 0x07,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(decode_matrix, expected_decode_matrix);

        // one parity error
        let decode_matrix = ec.make_decode_matrix(&[4]).unwrap();
        #[rustfmt::skip]
        let expected_decode_matrix: Vec<u8> = vec![
            0x47, 0xA7, 0x7A, 0xBA,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(decode_matrix, expected_decode_matrix);
    }

    // #[test]
    // This test is disabled because the gftbls generated varies by the version of libisal
    fn _make_table_from_matrix() {
        let k = 4;
        let m = 2;
        #[rustfmt::skip]
        let encode_matrix: Vec<u8> = vec![
            0x01, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x00,
            0x00, 0x00, 0x00, 0x01,
            0x47, 0xA7, 0x7A, 0xBA,
            0xA7, 0x47, 0xBA, 0x7A,
        ];
        #[rustfmt::skip]
        let expected_gftbls: Vec<u8> = vec![
            0x00, 0x47, 0x8E, 0xC9, 0x01, 0x46, 0x8F, 0xC8, 0x02, 0x45, 0x8C, 0xCB, 0x03, 0x44, 0x8D, 0xCA, 0x00, 0x04, 0x08, 0x0C, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24, 0x28, 0x2C, 0x30, 0x34, 0x38, 0x3C,
            0x00, 0xA7, 0x53, 0xF4, 0xA6, 0x01, 0xF5, 0x52, 0x51, 0xF6, 0x02, 0xA5, 0xF7, 0x50, 0xA4, 0x03, 0x00, 0xA2, 0x59, 0xFB, 0xB2, 0x10, 0xEB, 0x49, 0x79, 0xDB, 0x20, 0x82, 0xCB, 0x69, 0x92, 0x30,
            0x00, 0x7A, 0xF4, 0x8E, 0xF5, 0x8F, 0x01, 0x7B, 0xF7, 0x8D, 0x03, 0x79, 0x02, 0x78, 0xF6, 0x8C, 0x00, 0xF3, 0xFB, 0x08, 0xEB, 0x18, 0x10, 0xE3, 0xCB, 0x38, 0x30, 0xC3, 0x20, 0xD3, 0xDB, 0x28,
            0x00, 0xBA, 0x69, 0xD3, 0xD2, 0x68, 0xBB, 0x01, 0xB9, 0x03, 0xD0, 0x6A, 0x6B, 0xD1, 0x02, 0xB8, 0x00, 0x6F, 0xDE, 0xB1, 0xA1, 0xCE, 0x7F, 0x10, 0x5F, 0x30, 0x81, 0xEE, 0xFE, 0x91, 0x20, 0x4F,
            0x00, 0xA7, 0x53, 0xF4, 0xA6, 0x01, 0xF5, 0x52, 0x51, 0xF6, 0x02, 0xA5, 0xF7, 0x50, 0xA4, 0x03, 0x00, 0xA2, 0x59, 0xFB, 0xB2, 0x10, 0xEB, 0x49, 0x79, 0xDB, 0x20, 0x82, 0xCB, 0x69, 0x92, 0x30,
            0x00, 0x47, 0x8E, 0xC9, 0x01, 0x46, 0x8F, 0xC8, 0x02, 0x45, 0x8C, 0xCB, 0x03, 0x44, 0x8D, 0xCA, 0x00, 0x04, 0x08, 0x0C, 0x10, 0x14, 0x18, 0x1C, 0x20, 0x24, 0x28, 0x2C, 0x30, 0x34, 0x38, 0x3C,
            0x00, 0xBA, 0x69, 0xD3, 0xD2, 0x68, 0xBB, 0x01, 0xB9, 0x03, 0xD0, 0x6A, 0x6B, 0xD1, 0x02, 0xB8, 0x00, 0x6F, 0xDE, 0xB1, 0xA1, 0xCE, 0x7F, 0x10, 0x5F, 0x30, 0x81, 0xEE, 0xFE, 0x91, 0x20, 0x4F,
            0x00, 0x7A, 0xF4, 0x8E, 0xF5, 0x8F, 0x01, 0x7B, 0xF7, 0x8D, 0x03, 0x79, 0x02, 0x78, 0xF6, 0x8C, 0x00, 0xF3, 0xFB, 0x08, 0xEB, 0x18, 0x10, 0xE3, 0xCB, 0x38, 0x30, 0xC3, 0x20, 0xD3, 0xDB, 0x28,
        ];
        let parity_matrix = &encode_matrix[(k * k)..];
        let actual_gftbls = super::make_table_from_matrix(parity_matrix, k, m).unwrap();
        let sys_gftbles = vec![0_u8; k * m * 32];
        unsafe {
            let parity_ptr = parity_matrix.as_ptr() as *mut u8;
            let sys_tables_ptr = sys_gftbles.as_ptr() as *mut u8;
            erasure_isa_l_sys::ec_init_tables(4, 2, parity_ptr, sys_tables_ptr);
        }
        assert_eq!(sys_gftbles, actual_gftbls);
        assert_eq!(actual_gftbls.len(), expected_gftbls.len());
        assert_eq!(actual_gftbls, expected_gftbls);
    }
}
