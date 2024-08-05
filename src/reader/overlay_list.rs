use std::{
    borrow::Borrow,
    fmt::Debug,
    marker::PhantomData,
    pin::Pin,
    task::{ready, Context, Poll},
};

use crate::buf::DataReadBuf;

use super::{overlay_once::OverlayOnce, AsyncDataRead};

#[derive(Debug, Clone)]
pub struct OverlayList<T: Clone, C: Borrow<[T]>> {
    list: Vec<OverlayOnce<T, C>>,
    _p: PhantomData<T>,
}

impl<T: Clone, C: Borrow<[T]>> OverlayList<T, C> {
    pub fn new() -> Self {
        Self {
            list: vec![],
            _p: PhantomData,
        }
    }

    pub fn push(&mut self, item: OverlayOnce<T, C>) {
        self.list.push(item)
    }

    pub fn overlay_iter(&self) -> impl Iterator<Item = &OverlayOnce<T, C>> {
        self.list.iter()
    }
}

// NOTE: We can make this work without [`Unpin`] by never allow `self.list` to be moved out, but I not 100% sure it's true or not :)
impl<T: Clone + Unpin, C: Borrow<[T]> + Unpin> AsyncDataRead for OverlayList<T, C> {
    type Item = T;
    type Err = ();

    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let mut unused_buf = buf.take(buf.capacity());
        let mut cap = None;
        for s in self.list.iter_mut().rev() {
            let mut temp_buf = unused_buf.take(cap.unwrap_or(unused_buf.capacity()));
            let pre = temp_buf.filled().len();
            let pin = Pin::new(s);
            let next = ready!(pin.poll_read(cx, &mut temp_buf, pos))?;
            let wb = temp_buf.filled().len() - pre;
            cap = next.zip(cap).map(|(n, c)| n.min(c)).or(next).or(cap);
            if wb > 0 {
                return Poll::Ready(Ok(cap));
            }
        }

        // Empty source
        return Poll::Ready(Ok(None));
    }
}

#[cfg(test)]
mod tests {
    use crate::buf;

    use super::*;

    #[tokio::test]
    async fn basic() {
        let first = OverlayOnce::new(0, [1, 2, 3]);
        let mid = OverlayOnce::new(3, [4, 5, 6]);
        let last = OverlayOnce::new(6, [7, 8, 9]);

        let mut ol = OverlayList::new();
        ol.push(first);
        ol.push(last);
        ol.push(mid);

        let mut bf = buf::new::<10, _>();
        let next = ol.read_single_pass(0, &mut bf).await.expect("Read failed!");
        assert_eq!(next, Some(3));
        assert_eq!(bf.filled(), &[1, 2, 3]);
    }

    #[tokio::test]
    async fn overlapping() {
        let first = OverlayOnce::new(0, [1, 2, 3, 4]);
        let __mid = OverlayOnce::new(3, [4, 5, 6, 7]);
        let _last = OverlayOnce::new(6, [7, 8, 9, 10]);

        let mut ol = OverlayList::new();
        ol.push(first);
        ol.push(_last);
        ol.push(__mid);

        let mut bf = buf::new::<10, _>();
        let next = ol.read_single_pass(0, &mut bf).await.expect("Read failed!");
        assert_eq!(next, Some(3));
        assert_eq!(bf.filled(), &[1, 2, 3]);
    }

    #[tokio::test]
    async fn read_all_overlap() {
        let first = OverlayOnce::new(0, [1, 2, 3, 4]);
        let __mid = OverlayOnce::new(3, [4, 5, 6, 7]);
        let _last = OverlayOnce::new(6, [7, 8, 9, 10]);

        let mut ol = OverlayList::new();
        ol.push(first);
        ol.push(_last);
        ol.push(__mid);

        let mut bf = buf::new::<10, _>();
        let next = ol.read(0, &mut bf).await.expect("Read failed!");
        assert_eq!(next, None);
        assert_eq!(bf.filled(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }
}
