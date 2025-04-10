use http::{HeaderValue, Response};

pub trait IntoResponse<B> {
    fn into_response(self) -> Response<B>;
}

impl<B> IntoResponse<B> for Response<B> {
    fn into_response(self) -> Response<B> {
        self
    }
}

impl<'a, B> IntoResponse<B> for &'a str
where
    B: From<&'a str>,
{
    fn into_response(self) -> Response<B> {
        let mut resp = Response::new(B::from(self));
        resp.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain"),
        );
        resp
    }
}

impl<B> IntoResponse<B> for String
where
    B: From<String>,
{
    fn into_response(self) -> Response<B> {
        let mut resp = Response::new(B::from(self));
        resp.headers_mut().insert(
            http::header::CONTENT_TYPE,
            HeaderValue::from_static("text/plain"),
        );
        resp
    }
}
