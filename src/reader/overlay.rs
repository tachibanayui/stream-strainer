use std::{
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::{buf::DataReadBuf, or::Or};
use pin_project::pin_project;

use super::AsyncDataRead;

#[derive(Debug, Clone, Copy)]
#[pin_project]
pub struct OverlaySource<T, B, O>
where
    B: AsyncDataRead<Item = T>,
    O: AsyncDataRead<Item = T>,
{
    #[pin]
    base: B,

    #[pin]
    overlay: O,
}

impl<T, B, O> OverlaySource<T, B, O>
where
    B: AsyncDataRead<Item = T>,
    O: AsyncDataRead<Item = T>,
{
    pub fn new(base: B, overlay: O) -> Self {
        Self { base, overlay }
    }
}

impl<T, B, O> AsyncDataRead for OverlaySource<T, B, O>
where
    B: AsyncDataRead<Item = T>,
    O: AsyncDataRead<Item = T>,
{
    type Item = T;
    type Err = Or<B::Err, O::Err>;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let this = self.project();

        // Read the overlay data
        let prev = buf.filled().len();
        let overlay_next = match ready!(this.overlay.poll_read(cx, buf, pos)) {
            Err(err) => return Poll::Ready(Err(Or::T2(err))),
            Ok(x) => x,
        };
        let cur = buf.filled().len();
        let read_bytes = cur - prev;
        if read_bytes > 0 {
            return Poll::Ready(Ok(Some(pos + read_bytes)));
        }

        let unfilled = buf.capacity() - buf.filled().len();
        // Only fill the hole of overlay
        let limit = overlay_next.map(|x| x - pos).unwrap_or(unfilled);

        let mut new_buf = buf.take(limit);
        let base_next = match ready!(this.base.poll_read(cx, &mut new_buf, pos)) {
            Err(err) => return Poll::Ready(Err(Or::T1(err))),
            Ok(x) => x,
        };
        let read_bytes = new_buf.filled().len();
        if read_bytes > 0 {
            return Poll::Ready(Ok(Some(pos + read_bytes)));
        } else {
            return Poll::Ready(Ok(overlay_next.min(base_next)));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        buf::{self, DataReadBuf},
        reader::{overlay_once::OverlayOnce, AsyncDataRead},
    };

    #[tokio::test]
    async fn basic() {
        let mut source =
            OverlayOnce::new(0, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).overlay_once(5, &[100, 100]);
        let mut buf = buf::new::<10, _>();
        let next = source
            .read_single_pass(1, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(5));
        assert_eq!(buf.filled(), &mut [2, 3, 4, 5]);
    }
}
