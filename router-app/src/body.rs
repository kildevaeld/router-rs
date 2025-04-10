use bytes::Bytes;
use core::{pin::Pin, task::Poll};
use http_body_util::combinators::UnsyncBoxBody;

use crate::error::Error;

enum Inner {
    Reusable(Bytes),
    Streaming(UnsyncBoxBody<Bytes, Error>),
}

pub struct Body {
    inner: Inner,
}

impl Body {
    pub fn empty() -> Body {
        Body {
            inner: Inner::Reusable(Bytes::new()),
        }
    }

    pub fn from_streaming<B: http_body::Body>(inner: B) -> Body
    where
        B: Send + 'static,
        B::Error: Into<Error>,
        B::Data: Into<Bytes>,
    {
        use http_body_util::BodyExt;

        let boxed = inner
            .map_frame(|f| f.map_data(Into::into))
            .map_err(Into::into)
            .boxed_unsync();

        Body {
            inner: Inner::Streaming(boxed),
        }
    }
}

impl http_body::Body for Body {
    type Data = bytes::Bytes;

    type Error = Error;

    fn poll_frame(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        match self.inner {
            Inner::Reusable(ref mut bytes) => {
                let out = bytes.split_off(0);
                if out.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Ready(Some(Ok(http_body::Frame::data(out))))
                }
            }
            Inner::Streaming(ref mut body) => {
                Poll::Ready(futures_core::ready!(Pin::new(body).poll_frame(cx)))
            }
        }
    }

    fn size_hint(&self) -> http_body::SizeHint {
        match self.inner {
            Inner::Reusable(ref bytes) => http_body::SizeHint::with_exact(bytes.len() as u64),
            Inner::Streaming(ref body) => body.size_hint(),
        }
    }

    fn is_end_stream(&self) -> bool {
        match self.inner {
            Inner::Reusable(ref bytes) => bytes.is_empty(),
            Inner::Streaming(ref body) => body.is_end_stream(),
        }
    }
}

impl router::body::Body for Body {
    fn empty() -> Self {
        Body {
            inner: Inner::Reusable(Bytes::new()),
        }
    }
}

impl<'a> From<&'a str> for Body {
    fn from(value: &'a str) -> Self {
        value.as_bytes().to_vec().into()
    }
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        value.into_bytes().into()
    }
}

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Body {
            inner: Inner::Reusable(value.into()),
        }
    }
}

impl From<Bytes> for Body {
    fn from(value: Bytes) -> Self {
        Body {
            inner: Inner::Reusable(value),
        }
    }
}
