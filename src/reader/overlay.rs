use std::{
    io::SeekFrom,
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::{buf::DataReadBuf, or::Or};
use pin_project::pin_project;
use tokio::io::AsyncSeek;

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

    cur: u64,
    seeking: bool,
    desync: bool,
}

impl<T, B, O> OverlaySource<T, B, O>
where
    B: AsyncDataRead<Item = T>,
    O: AsyncDataRead<Item = T>,
{
    pub fn new(base: B, overlay: O) -> Self {
        Self {
            base,
            overlay,
            cur: 0,
            seeking: false,
            desync: false,
        }
    }
}

impl<T, B, O> AsyncDataRead for OverlaySource<T, B, O>
where
    B: AsyncDataRead<Item = T> + AsyncSeek,
    O: AsyncDataRead<Item = T> + AsyncSeek,
{
    type Item = T;
    type Err = Or<B::Err, O::Err>;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
    ) -> Poll<Result<Option<u64>, Self::Err>> {
        let mut this = self.project();
        if *this.seeking {
            ready!(this.base.as_mut().poll_complete(cx));
            *this.seeking = false;
            *this.desync = false;
        }

        // Read the overlay data
        let prev = buf.filled().len();
        let overlay_next = match ready!(this.overlay.poll_read(cx, buf)) {
            Err(err) => return Poll::Ready(Err(Or::R(err))),
            Ok(x) => x,
        };
        let cur = buf.filled().len();
        let wb = cur - prev;
        *this.cur += wb as u64;
        if wb > 0 {
            *this.desync = true;
            return Poll::Ready(Ok(Some(*this.cur)));
        }

        let unfilled = buf.capacity() - buf.filled().len();
        // Only fill the hole of overlay
        let limit = overlay_next
            .map(|x| x - *this.cur)
            .unwrap_or(unfilled as u64);

        let mut new_buf = buf.take(limit as usize);
        if *this.desync {
            this.base.as_mut().start_seek(SeekFrom::Start(*this.cur));
            *this.seeking = true;
        }

        let base_next = match ready!(this.base.as_mut().poll_read(cx, &mut new_buf)) {
            Err(err) => return Poll::Ready(Err(Or::L(err))),
            Ok(x) => x,
        };
        let wb = new_buf.filled().len();
        if wb > 0 {
            *this.cur += wb as u64;
            return Poll::Ready(Ok(Some(*this.cur)));
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
            OverlayOnce::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).overlay_once(5, &[100, 100]);
        let mut buf = buf::new::<10, _>();
        let next = source
            .read_single_pass(1, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(5));
        assert_eq!(buf.filled(), &mut [2, 3, 4, 5]);
    }
}
