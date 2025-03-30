use http::Response;

pub trait IntoResponse<B> {
    fn into_response(self) -> Response<B>;
}

impl<B> IntoResponse<B> for Response<B> {
    fn into_response(self) -> Response<B> {
        self
    }
}
