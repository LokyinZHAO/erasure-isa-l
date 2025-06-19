//! Interface to functions supporting erasure code encode and decode.
//!
//! This module defines the interface to optimized functions used in erasure
//! codes. Encode and decode of erasures in GF(2^8) are made by calculating the
//! dot product of the symbols (bytes in GF(2^8)) across a set of buffers and a
//! set of coefficients. Values for the coefficients are determined by the type
//! of erasure code. Using k general dot product means that any sequence of
//! coefficients may be used including erasure codes based on random coefficients.

/// Erasure code utility functions for encoding.
///
/// This module provides basic raw bindings to libisa-l functions
/// for erasure code encoding and decoding.
///
/// If you are looking for a higher-level interface, consider using
/// the [`erasure`](crate::erasure) module instead, which provides a more user-friendly API
/// for working with erasure codes.
pub mod ec {

    /// Initialize tables for fast Erasure Code encode and decode.
    ///
    /// Generates the expanded tables needed for fast encode or decode for erasure
    /// codes on blocks of data. 32 bytes is generated for each input coefficient.
    ///
    /// # Parameters
    ///
    /// * `k` - The number of vector sources or rows in the generator matrix for coding.
    /// * `rows` - The number of output vectors to concurrently encode/decode.
    /// * `a` - Pointer to sets of arrays of input coefficients used to encode or decode data.
    /// * `gf_tbls` - Pointer to start of space for concatenated output tables
    ///   generated from input coefficients. Must be of size 32*k*rows.
    pub fn init_tables(k: i32, rows: i32, a: &[u8], gf_tbls: &mut [u8]) {
        unsafe {
            erasure_isa_l_sys::ec_init_tables(k, rows, a.as_ptr() as *mut u8, gf_tbls.as_mut_ptr());
        }
    }

    /// Generate or decode erasure codes on blocks of data.
    ///
    /// Given a list of source data blocks, generate one or multiple blocks of
    /// encoded data as specified by a matrix of GF(2^8) coefficients. When given a
    /// suitable set of coefficients, this function will perform the fast generation
    /// or decoding of Reed-Solomon type erasure codes.
    ///
    /// This function determines what instruction sets are enabled and
    /// selects the appropriate version at runtime.
    ///
    /// # Parameters
    ///
    /// * `len` - Length of each block of data (vector) of source or dest data.
    /// * `k` - The number of vector sources or rows in the generator matrix for coding.
    /// * `rows` - The number of output vectors to concurrently encode/decode.
    /// * `gf_tbls` - Pointer to array of input tables generated from coding
    ///   coefficients in init_tables(). Must be of size 32*k*rows.
    /// * `data` - Array of pointers to source input buffers.
    /// * `code` - Array of pointers to coded output buffers.
    pub fn encode_data(
        len: i32,
        k: i32,
        rows: i32,
        gf_tbls: &[u8],
        data: &[*const u8],
        code: &mut [*mut u8],
    ) {
        unsafe {
            erasure_isa_l_sys::ec_encode_data(
                len,
                k,
                rows,
                gf_tbls.as_ptr() as *mut u8,
                data.as_ptr() as *mut *mut u8,
                code.as_mut_ptr(),
            );
        }
    }

    /// Generate update for encode or decode of erasure codes from single source.
    ///
    /// Given one source data block, update one or multiple blocks of encoded data as
    /// specified by a matrix of GF(2^8) coefficients. When given a suitable set of
    /// coefficients, this function will perform the fast generation or decoding of
    /// Reed-Solomon type erasure codes from one input source at a time.
    ///
    /// This function determines what instruction sets are enabled and selects the
    /// appropriate version at runtime.
    ///
    /// # Parameters
    ///
    /// * `len` - Length of each block of data (vector) of source or dest data.
    /// * `k` - The number of vector sources or rows in the generator matrix for coding.
    /// * `rows` - The number of output vectors to concurrently encode/decode.
    /// * `vec_i` - The vector index corresponding to the single input source.
    /// * `gf_tbls` - Pointer to array of input tables generated from coding
    ///   coefficients in init_tables(). Must be of size 32*k*rows.
    /// * `data` - Pointer to single input source used to update output parity.
    /// * `code` - Array of pointers to coded output buffers.
    pub fn encode_data_update(
        len: i32,
        k: i32,
        rows: i32,
        vec_i: i32,
        gf_tbls: &[u8],
        data: &[u8],
        code: &mut [*mut u8],
    ) {
        unsafe {
            erasure_isa_l_sys::ec_encode_data_update(
                len,
                k,
                rows,
                vec_i,
                gf_tbls.as_ptr() as *mut u8,
                data.as_ptr() as *mut u8,
                code.as_mut().as_mut_ptr(),
            );
        }
    }
}

/// Galois Field (GF) utility functions for erasure coding.
///
/// This module provides support functions used in GF(2^8) operations
/// for erasure coding.
///
/// This module provides basic raw bindings to libisa-l functions
/// for Galois Field operations. If you are looking for a higher-level
/// interface, consider using the [`erasure`](crate::erasure) module instead,
/// which provides a more user-friendly API for working with Galois Field operations.
pub mod gf {
    /// Single element GF(2^8) multiply.
    ///
    /// # Parameters
    ///
    /// * `a` - Multiplicand a
    /// * `b` - Multiplicand b
    ///
    /// # Returns
    ///
    /// Product of a and b in GF(2^8)
    pub fn mul(a: u8, b: u8) -> u8 {
        unsafe { erasure_isa_l_sys::gf_mul(a, b) }
    }

    /// Single element GF(2^8) inverse.
    ///
    /// # Parameters
    ///
    /// * `a` - Input element
    ///
    /// # Returns
    ///
    /// Field element b such that a x b = {1}
    pub fn inv(a: u8) -> u8 {
        unsafe { erasure_isa_l_sys::gf_inv(a) }
    }

    /// Generate a matrix of coefficients to be used for encoding.
    ///
    /// Vandermonde matrix example of encoding coefficients where high portion of
    /// matrix is identity matrix I and lower portion is constructed as 2^{i*(j-k+1)}
    /// i:{0,k-1} j:{k,m-1}. Commonly used method for choosing coefficients in
    /// erasure encoding but does not guarantee invertable for every sub matrix.
    ///
    /// # Parameters
    ///
    /// * `a` - [m x k] array to hold coefficients
    /// * `m` - Number of rows in matrix corresponding to srcs + parity
    /// * `k` - Number of columns in matrix corresponding to srcs
    pub fn gen_rs_matrix(a: &mut [u8], m: i32, k: i32) {
        unsafe {
            erasure_isa_l_sys::gf_gen_rs_matrix(a.as_mut_ptr(), m, k);
        }
    }

    /// Generate a Cauchy matrix of coefficients to be used for encoding.
    ///
    /// Cauchy matrix example of encoding coefficients where high portion of matrix
    /// is identity matrix I and lower portion is constructed as 1/(i + j) | i != j,
    /// i:{0,k-1} j:{k,m-1}. Any sub-matrix of a Cauchy matrix should be invertable.
    ///
    /// # Parameters
    ///
    /// * `a` - [m x k] array to hold coefficients
    /// * `m` - Number of rows in matrix corresponding to srcs + parity
    /// * `k` - Number of columns in matrix corresponding to srcs
    pub fn gen_cauchy1_matrix(a: &mut [u8], m: i32, k: i32) {
        unsafe {
            erasure_isa_l_sys::gf_gen_cauchy1_matrix(a.as_mut_ptr(), m, k);
        }
    }

    /// Invert a matrix in GF(2^8)
    ///
    /// Attempts to construct an n x n inverse of the input matrix.
    /// Will always destroy input matrix in process.
    ///
    /// # Parameters
    ///
    /// * `input` - Input matrix, destroyed by invert process
    /// * `output` - Output matrix such that \[input\] x \[output\] = \[I\] - identity matrix
    /// * `n` - Size of matrix \[nxn\]
    ///
    /// # Returns
    ///
    /// * `true` - On successful inversion
    /// * `false` - If input matrix is singular and cannot be inverted
    pub fn invert_matrix(input: &mut [u8], output: &mut [u8], n: i32) -> bool {
        let res = unsafe {
            erasure_isa_l_sys::gf_invert_matrix(input.as_mut_ptr(), output.as_mut_ptr(), n)
        };
        res == 0
    }

    /// GF(2^8) vector dot product, runs appropriate version.
    ///
    /// Does a GF(2^8) dot product across each byte of the input array and a constant
    /// set of coefficients to produce each byte of the output. Can be used for
    /// erasure coding encode and decode. Function requires pre-calculation of a
    /// 32*vlen byte constant array based on the input coefficients.
    ///
    /// This function determines what instruction sets are enabled and
    /// selects the appropriate version at runtime.
    ///
    /// # Parameters
    ///
    /// * `len` - Length of each vector in bytes. Must be >= 32.
    /// * `vlen` - Number of vector sources.
    /// * `gf_tbls` - Pointer to 32*vlen byte array of pre-calculated constants based
    ///   on the array of input coefficients.
    /// * `src` - Array of pointers to source inputs.
    /// * `dest` - Pointer to destination data array.
    pub fn vect_dot_prod(len: i32, vlen: i32, gf_tbls: &[u8], src: &[*const u8], dest: &mut [u8]) {
        unsafe {
            erasure_isa_l_sys::gf_vect_dot_prod(
                len,
                vlen,
                gf_tbls.as_ptr() as *mut u8,
                src.as_ptr() as *mut *mut u8,
                dest.as_mut_ptr(),
            );
        }
    }

    /// GF(2^8) vector multiply accumulate, runs appropriate version.
    ///
    /// Does a GF(2^8) multiply across each byte of input source with expanded
    /// constant and add to destination array. Can be used for erasure coding encode
    /// and decode update when only one source is available at a time. Function
    /// requires pre-calculation of a 32*vec byte constant array based on the input
    /// coefficients.
    ///
    /// This function determines what instruction sets are enabled and selects the
    /// appropriate version at runtime.
    ///
    /// # Parameters
    ///
    /// * `len` - Length of each vector in bytes. Must be >= 64.
    /// * `vec` - The number of vector sources or rows in the generator matrix
    ///   for coding.
    /// * `vec_i` - The vector index corresponding to the single input source.
    /// * `gf_tbls` - Pointer to array of input tables generated from coding
    ///   coefficients in init_tables(). Must be of size 32*vec.
    /// * `src` - Array of pointers to source inputs.
    /// * `dest` - Pointer to destination data array.
    pub fn vect_mad(len: i32, vec: i32, vec_i: i32, gf_tbls: &[u8], src: &[u8], dest: &mut [u8]) {
        unsafe {
            erasure_isa_l_sys::gf_vect_mad(
                len,
                vec,
                vec_i,
                gf_tbls.as_ptr() as *mut u8,
                src.as_ptr() as *mut u8,
                dest.as_mut_ptr(),
            );
        }
    }
}
