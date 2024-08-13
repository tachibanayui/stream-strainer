use std::{
    io::{self, ErrorKind, Seek, SeekFrom},
    task::{Context, Poll},
};

use extension_trait::extension_trait;

#[extension_trait]
pub impl SeekFromExt for SeekFrom {
    fn eval(self, cur: u64, len: u64) -> io::Result<u64> {
        let rs = match self {
            SeekFrom::Start(x) => x,
            SeekFrom::End(x) => {
                if x > 0 {
                    len + (x as u64)
                } else {
                    len.checked_sub(x.abs() as u64).ok_or_else(|| {
                        io::Error::new(ErrorKind::InvalidInput, "Input underflow!")
                    })?
                }
            }
            SeekFrom::Current(x) => {
                if x > 0 {
                    cur + (x as u64)
                } else {
                    cur.checked_sub(x.abs() as u64).ok_or_else(|| {
                        io::Error::new(ErrorKind::InvalidInput, "Input underflow!")
                    })?
                }
            }
        };

        Ok(rs)
    }
}

#[inline]
pub fn reschedule<T>(cx: &mut Context<'_>) -> Poll<T> {
    cx.waker().wake_by_ref();
    return Poll::Pending;
}
