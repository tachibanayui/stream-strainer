pub mod limit;
pub mod overlay;
pub mod overlay_list;
pub mod overlay_once;
pub mod shift;

use std::{
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::buf::DataReadBuf;
use limit::Limit;
use overlay::OverlaySource;
use overlay_once::OverlayOnce;
use shift::{ShiftLeft, ShiftRight};

async fn read_to_hole0<R, B>(
    reader: &mut R,
    buf: &mut B,
    pos: usize,
) -> Result<Option<usize>, R::Err>
where
    R: AsyncDataRead + Unpin,
    B: DataReadBuf<Item = R::Item>,
{
    let mut written = 0;
    loop {
        let mut clean_buf = buf.take(buf.capacity() - buf.filled().len());
        let next = reader
            .read_single_pass(pos + written, &mut clean_buf)
            .await?;
        let wb = clean_buf.filled().len();
        written += wb;
        if wb == 0 {
            return Ok(next);
        }
    }
}

pub trait AsyncDataRead {
    type Item;
    type Err;

    // Return type Some(x) = next available position at x, None = no available position
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>>;

    fn read_single_pass<'b, B: DataReadBuf<Item = Self::Item>>(
        &mut self,
        pos: usize,
        buf: &'b mut B,
    ) -> ReadFut<'_, 'b, Self, B>
    where
        Self: Sized + Unpin,
    {
        ReadFut::Pending(ReadFutData {
            reader: self,
            buf,
            pos,
        })
    }

    fn read<'b, B: DataReadBuf<Item = Self::Item>>(
        &mut self,
        pos: usize,
        buf: &'b mut B,
    ) -> impl Future<Output = Result<Option<usize>, Self::Err>>
    where
        Self: Sized + Unpin,
    {
        read_to_hole0(self, buf, pos)
    }

    fn overlay<O: AsyncDataRead<Item = Self::Item>>(
        self,
        other: O,
    ) -> OverlaySource<Self::Item, Self, O>
    where
        Self: Sized,
    {
        OverlaySource::new(self, other)
    }

    fn overlay_once<'a>(
        self,
        pos: usize,
        data: &'a [Self::Item],
    ) -> OverlaySource<Self::Item, Self, OverlayOnce<Self::Item, &'a [Self::Item]>>
    where
        Self: Sized,
        Self::Item: Clone,
    {
        self.overlay(OverlayOnce::new(pos, data))
    }

    fn shift_left(self, n: usize) -> ShiftLeft<Self>
    where
        Self: Sized,
    {
        ShiftLeft::new(self, n)
    }

    fn shift_right(self, n: usize) -> ShiftRight<Self>
    where
        Self: Sized,
    {
        ShiftRight::new(self, n)
    }

    fn limit(self, n: usize) -> Limit<Self>
    where
        Self: Sized,
    {
        Limit::new(self, n)
    }

    fn slice(self, start: usize, end: usize) -> Limit<ShiftLeft<Self>>
    where
        Self: Sized,
    {
        let len = end - start;
        self.shift_left(start).limit(len)
    }
}

impl<S: AsyncDataRead> AsyncDataRead for Pin<&mut S> {
    type Item = S::Item;
    type Err = S::Err;

    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
        pos: usize,
    ) -> Poll<Result<Option<usize>, Self::Err>> {
        let pin_deref = Pin::as_deref_mut(self);
        S::poll_read(pin_deref, cx, buf, pos)
    }
}

#[derive(Debug)]
pub enum ReadFut<'s, 'b, R, B> {
    Pending(ReadFutData<'s, 'b, R, B>),
    Done,
}

#[derive(Debug)]
pub struct ReadFutData<'s, 'b, R, B> {
    reader: &'s mut R,
    buf: &'b mut B,
    pos: usize,
}

impl<'s, 'b, 'i, R, B, T> Future for ReadFut<'s, 'b, R, B>
where
    R: AsyncDataRead<Item = T> + Unpin,
    B: DataReadBuf<Item = T>,
{
    type Output = Result<Option<usize>, R::Err>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use ReadFut::*;

        let fd = mem::replace(&mut *self, Done);
        let Pending(fd) = fd else {
            panic!("Poll on Completed future!");
        };

        let pin = Pin::new(&mut *fd.reader);
        let poll = pin.poll_read(cx, fd.buf, fd.pos);
        if let Poll::Pending = poll {
            *self = Pending(fd);
        }

        return poll;
    }
}
