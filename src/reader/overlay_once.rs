use std::{
    borrow::Borrow,
    io::{Seek, SeekFrom},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;
use tokio::io::AsyncSeek;

use super::AsyncDataRead;
use crate::{buf::DataReadBuf, utils::SeekFromExt};

#[derive(Debug, Clone, Copy)]
#[pin_project]
pub struct OverlayOnce<T, C: Borrow<[T]>> {
    pub cur: usize,
    pub data: C,
    _p: PhantomData<T>,
}

impl<T, C: Borrow<[T]>> AsyncSeek for OverlayOnce<T, C> {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> std::io::Result<()> {
        self.seek(position)?;
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Poll::Ready(Ok(self.cur as u64))
    }
}

impl<T, C: Borrow<[T]>> Seek for OverlayOnce<T, C> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cur = pos.eval(self.cur as u64, self.data.borrow().len() as u64)? as usize;
        Ok(self.cur as u64)
    }
}

impl<T, C: Borrow<[T]>> OverlayOnce<T, C> {
    pub fn new(data: C) -> Self {
        Self {
            cur: 0,
            data,
            _p: PhantomData,
        }
    }
}

impl<T, C: Borrow<[T]>> AsyncDataRead for OverlayOnce<T, C> {
    type Item = T;
    type Err = ();

    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut impl DataReadBuf<Item = Self::Item>,
    ) -> Poll<Result<Option<u64>, Self::Err>> {
        let wb = buf.put_slice_guard(&self.data.borrow()[self.cur..]);
        self.cur += wb;
        let rt = if wb == 0 { None } else { Some(self.cur) }.map(|x| x as u64);

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
        let mut source = OverlayOnce::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let mut buf = buf::new::<5, _>();
        let next = source
            .read_single_pass(1, &mut buf)
            .await
            .expect("Read failed!");

        assert_eq!(next, Some(6));
        assert_eq!(buf.filled(), &mut [2, 3, 4, 5, 6])
    }
}
