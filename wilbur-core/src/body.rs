use bytes::Bytes;

use crate::error::Error;

pub trait Body: http_body::Body + Sized {
    fn empty() -> Self;
}

pub async fn to_bytes<T: http_body::Body>(body: T) -> Result<Bytes, Error>
where
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    use http_body_util::BodyExt;

    BodyExt::collect(body)
        .await
        .map(|buf| buf.to_bytes())
        .map_err(Error::new)
}
