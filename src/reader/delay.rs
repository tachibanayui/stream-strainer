use std::{future::Future, io::SeekFrom, pin::Pin, task::ready, time::Duration};

use pin_project::pin_project;
use tokio::{
    io::AsyncSeek,
    time::{self, Instant},
};

use super::AsyncDataRead;

#[pin_project]
/// Induce a delay every time we poll the reader. Mostly for testing - making future not return immediately!
pub struct DelayReader<R> {
    duration: Duration,
    #[pin]
    delay: time::Sleep,
    #[pin]
    reader: R,
    seek_op: Option<SeekFrom>,
}

impl<R> DelayReader<R> {
    pub fn new(reader: R, duration: Duration) -> Self {
        Self {
            duration,
            reader,
            delay: time::sleep(duration),
            seek_op: None,
        }
    }
}

impl<R> AsyncDataRead for DelayReader<R>
where
    R: AsyncDataRead,
{
    type Item = R::Item;
    type Err = R::Err;

    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut impl crate::buf::DataReadBuf<Item = Self::Item>,
    ) -> std::task::Poll<Result<Option<u64>, Self::Err>> {
        let mut this = self.project();
        ready!(this.delay.as_mut().poll(cx));
        this.delay.reset(Instant::now() + *this.duration);
        this.reader.poll_read(cx, buf)
    }
}

impl<R: AsyncSeek> AsyncSeek for DelayReader<R> {
    fn start_seek(self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        let this = self.project();
        *this.seek_op = Some(position);
        Ok(())
    }

    fn poll_complete(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<u64>> {
        let this = self.project();
        this.reader.poll_complete(cx)
    }
}
