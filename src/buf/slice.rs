use super::{DataReadBuf, View};

#[derive(Debug)]
pub struct DataReaderSlice<'p, P: View> {
    parent: &'p mut P,
    pos: usize,
    len: usize,
}

impl<'p, P: View> DataReaderSlice<'p, P> {
    pub fn new(parent: &'p mut P, pos: usize, len: usize) -> Self {
        Self { parent, pos, len }
    }
}

impl<'p, P: View> View for DataReaderSlice<'p, P> {
    unsafe fn set_init(&mut self, value: usize) {
        self.parent.set_init(value + self.pos)
    }

    fn set_filled(&mut self, value: usize) {
        self.parent.set_filled(value + self.pos)
    }
}

impl<'p, P: View> DataReadBuf for DataReaderSlice<'p, P> {
    type Item = P::Item;

    fn capacity(&self) -> usize {
        self.len
    }

    fn filled(&self) -> &[Self::Item] {
        let pf = self.parent.filled();
        let end = pf.len().min(self.pos + self.len);
        if pf.len() < self.pos {
            return &pf[0..0];
        }
        &pf[self.pos..end]
    }

    fn filled_mut(&mut self) -> &mut [Self::Item] {
        let pf = self.parent.filled_mut();
        let end = pf.len().min(self.pos + self.len);
        if pf.len() < self.pos {
            return &mut pf[0..0];
        }
        &mut pf[self.pos..end]
    }

    fn shrink(&mut self, count: usize) {
        debug_assert!(
            self.filled().len() - self.pos >= count,
            "Shrink more than filled!"
        );
        self.parent.shrink(count);
    }

    fn take(&mut self, n: usize) -> impl DataReadBuf<Item = Self::Item> {
        DataReaderSlice {
            len: n,
            pos: self.filled().len(),
            parent: self,
        }
    }

    fn put_slice(&mut self, data: &[Self::Item]) {
        let unfilled = self.capacity() - self.filled().len();
        assert!(unfilled >= data.len(), "data overflow buffer!");
        self.parent.put_slice(data);
    }
}

#[cfg(test)]
pub mod tests {
    use crate::buf::{DataReadBuf, DataReadBufImpl};

    #[test]
    fn take() {
        let mut buf = DataReadBufImpl::new_stack_alloc::<10>();
        buf.put_slice(&[1, 2, 3, 4, 5]);
        let mut slice = buf.take(4);
        slice.put_slice(&[6]);
        assert_eq!(slice.filled(), &[6]);
        assert_eq!(slice.filled_mut(), &mut [6]);

        drop(slice);
        assert_eq!(buf.filled(), &[1, 2, 3, 4, 5, 6])
    }
}
