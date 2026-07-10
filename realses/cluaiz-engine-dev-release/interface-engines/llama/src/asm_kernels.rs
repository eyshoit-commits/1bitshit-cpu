//! ═══════════════════════════════════════════════════════════════════════
//!  Llama Kernel: Bare-Metal Assembly Kernels (AVX-512 / AVX2)
//! ═══════════════════════════════════════════════════════════════════════

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

/// Hardware-aware mathematical dispatch interface.
pub trait BareMetalMath {
    unsafe fn ternary_dot_product(
        packed_weights: *const u8,
        activations: *const i8,
        output: *mut i32,
        count: usize,
    ) -> Result<(), &'static str>;
}

/// Industrial-standard ternary kernel using `_mm256_maddubs_epi16`.
pub struct Avx2MaddubsKernel;

#[cfg(target_arch = "x86_64")]
impl BareMetalMath for Avx2MaddubsKernel {
    unsafe fn ternary_dot_product(
        packed_weights: *const u8,
        activations: *const i8,
        output: *mut i32,
        count: usize,
    ) -> Result<(), &'static str> {
        if count == 0 { return Ok(()); }
        if !count.is_multiple_of(32) {
            return Err("count must be a multiple of 32 for AVX2 maddubs path");
        }

        let mask_lo = _mm256_set1_epi8(0x03_i8);
        let ones    = _mm256_set1_epi16(1);
        let mut accumulator = _mm256_setzero_si256();

        for block_idx in 0..(count / 32) {
            let packed_offset = block_idx * 8;
            let act_offset    = block_idx * 32;

            let packed_256 = _mm256_set1_epi64x(
                std::ptr::read_unaligned(packed_weights.add(packed_offset) as *const i64)
            );
            
            let idx0 = _mm256_and_si256(packed_256, mask_lo);
            let idx1 = _mm256_and_si256(_mm256_srli_epi16(packed_256, 2), mask_lo);
            let idx2 = _mm256_and_si256(_mm256_srli_epi16(packed_256, 4), mask_lo);
            let idx3 = _mm256_and_si256(_mm256_srli_epi16(packed_256, 6), mask_lo);

            let merged_lo = _mm256_unpacklo_epi8(idx0, idx1);
            let merged_hi = _mm256_unpacklo_epi8(idx2, idx3);
            let unpacked_weights = _mm256_unpacklo_epi16(merged_lo, merged_hi);

            let acts = _mm256_loadu_si256(activations.add(act_offset) as *const __m256i);
            let products = _mm256_maddubs_epi16(unpacked_weights, acts);
            let widened = _mm256_madd_epi16(products, ones);
            accumulator = _mm256_add_epi32(accumulator, widened);
        }

        _mm256_storeu_si256(output as *mut __m256i, accumulator);
        Ok(())
    }
}
