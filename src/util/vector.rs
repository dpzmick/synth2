use std::ops::{Mul, Add};

use simd;

pub trait Vectorizable:  Mul<Output=Self> + Add<Output = Self> + Clone + Default {
    type SimdType: simd::Simd + Mul<Output=Self::SimdType>;

    fn vector_size() -> usize;
    fn load(arr: &[Self], idx: usize) -> Self::SimdType;
    fn extract(this: &Self::SimdType, element: u32) -> Self;
}

// this appears to be available all the time, but we don't want to use it if we
// have AVX
#[cfg(not(target_feature = "avx"))]
impl Vectorizable for f32 {
    type SimdType = simd::f32x4;

    #[inline]
    fn vector_size() -> usize { 4 }

    #[inline]
    fn load(arr: &[Self], idx: usize) -> Self::SimdType
    {
        Self::SimdType::load(arr, idx)
    }

    #[inline]
    fn extract(this: &Self::SimdType, element: u32) -> Self
    {
        this.extract(element)
    }
}

#[cfg(target_feature = "avx")]
use simd::x86::avx;

#[cfg(target_feature = "avx")]
impl Vectorizable for f32 {
    type SimdType = avx::f32x8;

    #[inline]
    fn vector_size() -> usize { 8 }

    #[inline]
    fn load(arr: &[Self], idx: usize) -> Self::SimdType
    {
        Self::SimdType::load(arr, idx)
    }

    #[inline]
    fn extract(this: &Self::SimdType, element: u32) -> Self
    {
        this.extract(element)
    }
}

// TODO this actually slows everything down?
#[cfg(target_feature = "avx")]
impl Vectorizable for i64 {
    type SimdType = avx::i64x4;

    #[inline]
    fn vector_size() -> usize { 4 }

    #[inline]
    fn load(arr: &[Self], idx: usize) -> Self::SimdType
    {
        Self::SimdType::load(arr, idx)
    }

    #[inline]
    fn extract(this: &Self::SimdType, element: u32) -> Self
    {
        this.extract(element)
    }
}
