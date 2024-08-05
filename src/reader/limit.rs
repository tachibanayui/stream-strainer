use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use super::AsyncDataRead;
use crate::buf::DataReadBuf;
use pin_project::pin_project;

#[derive(Debug, Clone, Copy)]
#[pin_project]
pub struct Limit<S>(#[pin] S, usize);

impl<S> Limit<S> {
    pub fn new(data: S, n: usize) -> Self {
        Self(data, n)
    }
}

impl<S: AsyncDataRead> AsyncDataRead for Limit<S> {
    type Item = S::Item;
    type Err = S::Err;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let mut new_buf = buf.take(self.1);
        let this = self.project();
        let poll = ready!(this.0.poll_read(cx, &mut new_buf, pos))?.filter(|x| x < this.1);
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
    async fn basic() {
        let source = OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut ss = Limit::new(source, 3);
        let mut buf = buf::new::<10, _>();
        let next = ss
            .read_single_pass(0, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, None);
        assert_eq!(buf.filled(), &[1, 2, 3]);
    }
}
