pub mod default_impl;
pub mod slice;

use std::mem::MaybeUninit;

pub use self::{default_impl::DataReadBufImpl, slice::DataReaderSlice};

/// Create new owned buffer with [`new`] or [`new_boxed`]
pub trait DataReadBuf {
    type Item;

    fn filled(&self) -> &[Self::Item];
    fn shrink(&mut self, count: usize);
    fn capacity(&self) -> usize;
    fn filled_mut(&mut self) -> &mut [Self::Item];
    fn take(&mut self, n: usize) -> impl DataReadBuf<Item = Self::Item> + '_;

    // Same req as tokio ReadBuf
    fn put_slice(&mut self, data: &[Self::Item]);
    fn put_slice_guard(&mut self, data: &[Self::Item]) -> usize {
        let remaining = self.capacity() - self.filled().len();
        let writable = remaining.min(data.len());
        self.put_slice(&data[..writable]);
        return writable;
    }
}

pub fn new<const N: usize, T>() -> impl DataReadBuf<Item = T>
where
    T: Clone,
{
    DataReadBufImpl::new_stack_alloc::<N>()
}

pub fn new_boxed<T>(size: usize) -> impl DataReadBuf<Item = T>
where
    T: Clone,
{
    DataReadBufImpl::new_heap_alloc(size)
}

pub trait View: DataReadBuf {
    unsafe fn set_init(&mut self, value: usize);
    fn set_filled(&mut self, value: usize);
}

#[deprecated]
pub fn new_inner<const N: usize, T>() -> [MaybeUninit<T>; N] {
    [const { MaybeUninit::uninit() }; N]
}
