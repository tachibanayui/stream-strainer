use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use super::AsyncDataRead;
use crate::buf::DataReadBuf;
use pin_project::pin_project;

#[derive(Debug, Clone, Copy)]
#[pin_project]
pub struct ShiftLeft<S>(#[pin] S, usize);

impl<S> ShiftLeft<S> {
    pub fn new(data: S, n: usize) -> Self {
        Self(data, n)
    }
}

impl<S: AsyncDataRead> AsyncDataRead for ShiftLeft<S> {
    type Item = S::Item;
    type Err = S::Err;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let this = self.project();
        let poll = ready!(this.0.poll_read(cx, buf, pos + *this.1))?
            .map(|x| x.checked_sub(*this.1).unwrap_or(0));

        Poll::Ready(Ok(poll))
    }
}

#[derive(Debug, Clone, Copy)]
#[pin_project]
pub struct ShiftRight<S>(#[pin] S, usize);

impl<S> ShiftRight<S> {
    pub fn new(data: S, n: usize) -> Self {
        Self(data, n)
    }
}

impl<S: AsyncDataRead> AsyncDataRead for ShiftRight<S> {
    type Item = S::Item;
    type Err = S::Err;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let Some(offset) = pos.checked_sub(self.1) else {
            return Poll::Ready(Ok(Some(self.1 - pos)));
        };

        let this = self.project();
        let poll = ready!(this.0.poll_read(cx, buf, offset))?.map(|x| x + *this.1);

        Poll::Ready(Ok(poll))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        buf::{self, DataReadBuf},
        reader::{overlay_once::OverlayOnce, AsyncDataRead},
    };

    use super::*;

    #[tokio::test]
    async fn left_basic() {
        let source = OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut ss = ShiftLeft::new(source, 1);
        let mut buf = buf::new::<3, _>();
        let next = ss
            .read_single_pass(0, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(3));
        assert_eq!(buf.filled(), &[2, 3, 4]);
    }

    #[tokio::test]
    async fn left_oob_left() {
        let source = OverlayOnce::new(10, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut ss = ShiftLeft::new(source, 1);
        let mut buf = buf::new::<10, _>();
        let next = ss
            .read_single_pass(0, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(9));
        assert_eq!(buf.filled(), &[]);
    }

    #[tokio::test]
    async fn right_basic() {
        let source = OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut ss = ShiftRight::new(source, 1);
        let mut buf = buf::new::<3, _>();
        let next = ss
            .read_single_pass(0, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(1));
        assert_eq!(buf.filled(), &[]);
    }
}
