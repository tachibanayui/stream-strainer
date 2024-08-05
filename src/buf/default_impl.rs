use super::{slice::DataReaderSlice, DataReadBuf, View};
use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    mem::{transmute, MaybeUninit},
};

#[derive(Debug, Clone)]
pub struct DataReadBufImpl<T, C: BorrowMut<[MaybeUninit<T>]>> {
    data: C,
    filled: usize,
    inited: usize,
    _p: PhantomData<T>,
}

impl<T> DataReadBufImpl<T, [MaybeUninit<T>; 10]> {
    pub fn new_stack_alloc<const N: usize>() -> DataReadBufImpl<T, [MaybeUninit<T>; N]> {
        // Safety: we don't promise anything is initialzied here
        unsafe { DataReadBufImpl::new_unchecked([const { MaybeUninit::uninit() }; N], 0, 0) }
    }

    pub fn new_heap_alloc(size: usize) -> DataReadBufImpl<T, Box<[MaybeUninit<T>]>> {
        let v: Vec<MaybeUninit<T>> = (0..size).map(|_| MaybeUninit::uninit()).collect();
        // Safety: we don't promise anything is initialzied here
        unsafe { DataReadBufImpl::new_unchecked(v.into_boxed_slice(), 0, 0) }
    }
}

impl<T, C: BorrowMut<[MaybeUninit<T>]>> DataReadBufImpl<T, C> {
    /// Safety: Caller must make sure the slice returned by C must be initialized correctly up to `inited`!
    pub unsafe fn new_unchecked(data: C, filled: usize, inited: usize) -> Self {
        Self {
            data,
            filled,
            inited,
            _p: PhantomData,
        }
    }
}

impl<'a, T> DataReadBufImpl<T, &'a mut [MaybeUninit<T>]> {
    /// NOTE: Caller should make sure the entire slice is uninit or else memory will leak at initialized region
    pub fn slice_uninit(buf: &'a mut [MaybeUninit<T>]) -> Self {
        // Safety: we don't promise anything is initialzied here
        unsafe { Self::new_unchecked(buf, 0, 0) }
    }
}

impl<T, C: BorrowMut<[MaybeUninit<T>]>> View for DataReadBufImpl<T, C>
where
    T: Clone,
{
    unsafe fn set_init(&mut self, value: usize) {
        self.inited = value
    }

    fn set_filled(&mut self, value: usize) {
        self.filled = value
    }
}

impl<T, C: BorrowMut<[MaybeUninit<T>]>> DataReadBufImpl<T, C> {
    fn unfilled_mut(&mut self) -> &mut [T] {
        let region = &mut self.data.borrow_mut()[self.filled..self.inited];
        unsafe { transmute(region) }
    }

    fn push_uninit(&mut self, item: T) {
        self.data.borrow_mut()[self.filled].write(item);
        self.inited += 1;
        self.filled += 1;
    }

    pub fn take(&mut self, n: usize) -> DataReaderSlice<'_, Self>
    where
        T: Clone,
    {
        self.inited = self.filled;
        DataReaderSlice::new(self, self.filled, n)
    }
}

impl<T, C: BorrowMut<[MaybeUninit<T>]>> Drop for DataReadBufImpl<T, C> {
    fn drop(&mut self) {
        for x in &mut self.data.borrow_mut()[..self.inited] {
            unsafe { x.assume_init_drop() }
        }
    }
}

impl<T, C> DataReadBuf for DataReadBufImpl<T, C>
where
    T: Clone,
    C: BorrowMut<[MaybeUninit<T>]> + Borrow<[MaybeUninit<T>]>,
{
    type Item = T;

    fn capacity(&self) -> usize {
        self.data.borrow().len()
    }

    fn filled(&self) -> &[T] {
        let data = &self.data.borrow()[..self.filled];
        unsafe { transmute(data) }
    }

    fn filled_mut(&mut self) -> &mut [T] {
        let data = &mut self.data.borrow_mut()[..self.filled];
        unsafe { transmute(data) }
    }

    // Note: Item are cloned into buffer, if you need to preserve reference semantics, use Rc or Arc
    fn put_slice(&mut self, data: &[T]) {
        let uf = self.unfilled_mut();
        let mut iter = data.iter();
        let mut uf_iter = uf.iter_mut();
        let mut written = 0;

        loop {
            let Some(ufn) = uf_iter.next() else {
                break;
            };

            let Some(item) = iter.next() else {
                self.filled += written;
                return;
            };

            *ufn = item.clone();
            written += 1;
        }

        self.filled += written;

        loop {
            let Some(item) = iter.next() else {
                return;
            };

            self.push_uninit(item.clone())
        }
    }

    fn shrink(&mut self, count: usize) {
        self.filled -= count;
    }

    fn take(&mut self, n: usize) -> impl DataReadBuf<Item = T> + '_ {
        Self::take(self, n)
    }
}

#[cfg(test)]
pub mod tests {
    use std::{mem::MaybeUninit, rc::Rc};

    use crate::buf::DataReadBuf;

    use super::DataReadBufImpl;

    #[test]
    fn memory_test() {
        // Setup test: we check for leak using the last element still have strong count of 2 after buf is dropped
        // The test proofs that we don't call drop on uninitized data
        let mut inner: Vec<_> = (0..9).map(|_| MaybeUninit::uninit()).collect();
        let last = Rc::new(9);
        inner.push(MaybeUninit::new(last.clone()));
        let mut buf = DataReadBufImpl::slice_uninit(inner.as_mut_slice());

        let data = Rc::new(0);
        buf.put_slice(&[data.clone()]);

        assert_eq!(Rc::strong_count(&data), 2);
        drop(buf);
        assert_eq!(Rc::strong_count(&data), 1);
        assert_eq!(Rc::strong_count(&last), 2);
    }

    #[test]
    fn fill() {
        let mut buf = DataReadBufImpl::new_stack_alloc::<10>();
        buf.put_slice(&[1, 2, 3, 4, 5]);
        buf.put_slice_guard(&[6, 7, 8, 9, 10, 11]);
        assert_eq!(buf.filled(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(buf.filled_mut(), &mut [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn take() {
        let mut buf = DataReadBufImpl::new_stack_alloc::<10>();
        buf.put_slice(&[1, 2, 3, 4, 5]);
        let mut slice = buf.take(4);
        slice.put_slice(&[6]);
        assert_eq!(buf.filled(), &[1, 2, 3, 4, 5, 6]);
    }
}
