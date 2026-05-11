//! Extra traits for aligned

use core::mem;

use aligned::{Aligned, Alignment};
use as_slice::{AsMutSlice, AsSlice};

/// Create an aligned slice from an aligned array
pub trait AsAlignedSlice {
    /// Slice element
    type Element;
    /// Slice alignment
    type Alignment: Alignment;

    /// Create the slice
    fn as_aligned_slice(&self) -> &Aligned<Self::Alignment, [Self::Element]>;
}

/// Create an aligned slice from an aligned array
pub trait AsMutAlignedSlice {
    /// Slice element
    type Element;
    /// Slice alignment
    type Alignment: Alignment;

    /// Create the slice
    fn as_mut_aligned_slice(&mut self) -> &mut Aligned<Self::Alignment, [Self::Element]>;
}

impl<A, T> AsAlignedSlice for Aligned<A, T>
where
    A: Alignment,
    T: AsSlice,
{
    type Element = T::Element;
    type Alignment = A;

    #[inline]
    fn as_aligned_slice(&self) -> &Aligned<A, [T::Element]> {
        unsafe { mem::transmute(T::as_slice(&**self)) }
    }
}

impl<A, T> AsMutAlignedSlice for Aligned<A, T>
where
    A: Alignment,
    T: AsMutSlice,
{
    type Element = T::Element;
    type Alignment = A;

    #[inline]
    fn as_mut_aligned_slice(&mut self) -> &mut Aligned<A, [T::Element]> {
        unsafe { mem::transmute(T::as_mut_slice(&mut **self)) }
    }
}
