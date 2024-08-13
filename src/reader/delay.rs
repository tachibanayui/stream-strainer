use std::{future::Future, task::ready, time::Duration};

use pin_project::pin_project;
use tokio::time::{self, Instant, Sleep};

use super::AsyncDataRead;

#[pin_project]
/// Induce a delay every time we poll the reader. Mostly for testing - making future not return immediately!
pub struct DelayReader<R> {
    duration: Duration,
    #[pin]
    delay: time::Sleep,
    #[pin]
    reader: R,
}

impl<R> DelayReader<R> {
    pub fn new(reader: R, duration: Duration) -> Self {
        Self {
            duration,
            reader,
            delay: tokio::time::sleep(duration),
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
        pos: usize,
    ) -> std::task::Poll<Result<Option<usize>, Self::Err>> {
        let mut this = self.project();
        ready!(this.delay.as_mut().poll(cx));
        this.delay.reset(Instant::now() + *this.duration);
        this.reader.poll_read(cx, buf, pos)
    }
}
