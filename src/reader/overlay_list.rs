use std::{
    fmt::Debug,
    pin::{pin, Pin},
    task::{ready, Context, Poll},
};

use pin_project::pin_project;

use super::AsyncDataRead;
use crate::buf::DataReadBuf;

#[derive(Debug)]
#[pin_project]
pub struct OverlayList2<F>
where
    F: Producer,
{
    tf: F,
    iter: Option<F::Iter>,
    cap: Option<usize>,
    #[pin]
    reader: Option<F::Reader>,
}

impl<F> AsyncDataRead for OverlayList2<F>
where
    F: Producer,
{
    type Item = F::Item;
    type Err = F::Err;

    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let this = self.project();
        let mut unused_buf = buf.take(buf.capacity());
        let mut temp_buf = unused_buf.take(this.cap.unwrap_or(unused_buf.capacity()));

        if let Some(r) = this.reader.as_pin_mut() {
            let next = ready!(r.poll_read(cx, &mut temp_buf, pos))?;
        }

        if this.iter.is_none() {
            *this.iter = Some(this.tf.produce_iter());
        }

        let iter = this.iter.as_mut().unwrap().next();

        // let mut cap = None;

        // for s in self.tf.produce_iter() {
        //     let mut temp_buf = unused_buf.take(cap.unwrap_or(unused_buf.capacity()));
        //     let pre = temp_buf.filled().len();
        //     let pin = pin!(s);
        //     let next = ready!(pin.poll_read(cx, &mut temp_buf, pos))?;
        //     let wb = temp_buf.filled().len() - pre;
        //     cap = next.zip(cap).map(|(n, c)| n.min(c)).or(next).or(cap);
        //     if wb > 0 {
        //         return Poll::Ready(Ok(cap));
        //     }
        // }

        todo!()
    }
}

#[derive(Debug)]
#[pin_project]
pub struct OverlayList<F> {
    tf: F,
    // iter: Option<F::Iter>,
    // reader: Option<F::Reader>,
}

impl<F> OverlayList<F> {
    pub fn new(tf: F) -> Self {
        Self { tf }
    }
}

pub trait Producer {
    type Err;
    type Item;
    type Reader: AsyncDataRead<Item = Self::Item, Err = Self::Err>;
    type Iter: Iterator<Item = Self::Reader> + DoubleEndedIterator;
    fn produce_iter(&mut self) -> Self::Iter;
}

// NOTE: We can make this work without [`Unpin`] by never allow `self.list` to be moved out, but I not 100% sure it's true or not :)
impl<F> AsyncDataRead for OverlayList<F>
where
    F: Producer,
{
    type Item = F::Item;
    type Err = F::Err;

    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let mut unused_buf = buf.take(buf.capacity());
        // let mut cap = None;

        // for s in self.tf.produce_iter() {
        //     let mut temp_buf = unused_buf.take(cap.unwrap_or(unused_buf.capacity()));
        //     let pre = temp_buf.filled().len();
        //     let pin = pin!(s);
        //     let next = ready!(pin.poll_read(cx, &mut temp_buf, pos))?;
        //     let wb = temp_buf.filled().len() - pre;
        //     cap = next.zip(cap).map(|(n, c)| n.min(c)).or(next).or(cap);
        //     if wb > 0 {
        //         return Poll::Ready(Ok(cap));
        //     }
        // }

        // Empty source
        return Poll::Ready(Ok(None));
    }
}

#[cfg(test)]
mod tests {
    use crate::{buf, reader::overlay_once::OverlayOnce};

    use super::*;

    // #[tokio::test]
    // async fn basic() {
    //     let first = OverlayOnce::new(0, [1, 2, 3]).delay(Duration::from_millis(100));
    //     let mid = OverlayOnce::new(3, [4, 5, 6]).delay(Duration::from_millis(100));
    //     let last = OverlayOnce::new(6, [7, 8, 9]).delay(Duration::from_millis(100));
    //     let mut v = [first, mid, last];

    //     let p = test(v.as_mut_slice());
    //     fn test<P: Producer>(p: P) -> P {
    //         p
    //     }

    //     let mut ol = OverlayList::new(p);
    //     let mut bf = buf::new::<10, _>();
    //     let next = ol.read_single_pass(0, &mut bf).await.expect("Read failed!");
    //     assert_eq!(next, Some(3));
    //     assert_eq!(bf.filled(), &[1, 2, 3]);
    // }

    // #[tokio::test]
    // async fn overlapping() {
    //     let first = OverlayOnce::new(0, [1, 2, 3, 4]);
    //     let __mid = OverlayOnce::new(3, [4, 5, 6, 7]);
    //     let _last = OverlayOnce::new(6, [7, 8, 9, 10]);
    //     let mut v = [first, __mid, _last];
    //     let mut ol = OverlayList::new(|| v.into_iter());

    //     let mut bf = buf::new::<10, _>();
    //     let next = ol.read_single_pass(0, &mut bf).await.expect("Read failed!");
    //     assert_eq!(next, Some(3));
    //     assert_eq!(bf.filled(), &[1, 2, 3]);
    // }

    // #[tokio::test]
    // async fn read_all_overlap() {
    //     let first = OverlayOnce::new(0, [1, 2, 3, 4]);
    //     let __mid = OverlayOnce::new(3, [4, 5, 6, 7]);
    //     let _last = OverlayOnce::new(6, [7, 8, 9, 10]);
    //     let mut v = [first, __mid, _last];
    //     let mut ol = OverlayList::new(|| v.into_iter());

    //     let mut bf = buf::new::<10, _>();
    //     let next = ol.read(0, &mut bf).await.expect("Read failed!");
    //     assert_eq!(next, None);
    //     assert_eq!(bf.filled(), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    // }
}
