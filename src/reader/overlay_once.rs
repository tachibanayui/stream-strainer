use std::{
    borrow::Borrow,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use super::AsyncDataRead;
use crate::buf::DataReadBuf;

#[derive(Debug, Clone, Copy)]
pub struct OverlayOnce<T, C: Borrow<[T]>> {
    pub pos: usize,
    pub data: C,
    _p: PhantomData<T>,
}

impl<T, C: Borrow<[T]>> OverlayOnce<T, C> {
    pub fn new(pos: usize, data: C) -> Self {
        Self {
            pos,
            data,
            _p: PhantomData,
        }
    }
}

impl<T, C: Borrow<[T]>> AsyncDataRead for OverlayOnce<T, C> {
    type Item = T;
    type Err = ();

    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        if self.pos > pos {
            return Poll::Ready(Ok(Some(self.pos)));
        }

        let idx = pos - self.pos;
        if idx >= self.data.borrow().len() {
            return Poll::Ready(Ok(None));
        }

        let wb = buf.put_slice_guard(&self.data.borrow()[idx..]);
        let rt = if idx + wb == self.data.borrow().len() {
            None
        } else {
            Some(idx + wb)
        };

        return Poll::Ready(Ok(rt));
    }
}

#[cfg(test)]
mod tests {
    use super::OverlayOnce;
    use crate::{
        buf::{self, DataReadBuf},
        reader::AsyncDataRead,
    };

    #[tokio::test]
    async fn basic() {
        let mut source = OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut buf = buf::new::<5, _>();
        let next = source
            .read_single_pass(1, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(6));
        assert_eq!(buf.filled(), &mut [2, 3, 4, 5, 6])
    }

    #[tokio::test]
    async fn no_head() {
        let mut source = OverlayOnce::new(5, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut buf = buf::new::<10, _>();
        let next = source
            .read_single_pass(0, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(5));
        assert_eq!(buf.filled(), &mut [])
    }

    #[tokio::test]
    async fn right_oob() {
        let mut source = OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut buf = buf::new::<10, _>();
        let next = source
            .read_single_pass(69, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, None);
        assert_eq!(buf.filled(), &mut [])
    }

    #[tokio::test]
    async fn right_oob_partial() {
        let mut source = OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut buf = buf::new::<10, _>();
        let next = source
            .read_single_pass(0, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, None);
        assert_eq!(buf.filled(), &mut [1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    }
}
