use std::convert::Infallible;

use heather::HBoxFuture;
use http::{HeaderMap, Request, Uri, request::Parts};

#[derive(Debug)]
pub struct FromRequestError(Box<dyn core::error::Error + Send + Sync>);

impl core::fmt::Display for FromRequestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::error::Error for FromRequestError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&*self.0)
    }
}

pub trait FromRequestParts<C>: Sized {
    type Error;

    type Future<'a>: Future<Output = Result<Self, Self::Error>>
    where
        C: 'a;

    fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a>;
}

pub trait FromRequest<B, C>: Sized {
    type Error;

    type Future<'a>: Future<Output = Result<Self, Self::Error>>
    where
        C: 'a,
        B: 'a;

    fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a>;
}

impl<C: 'static, B: 'static> FromRequest<B, C> for Request<B> {
    type Error = Infallible;
    type Future<'a> = core::future::Ready<Result<Self, Self::Error>>;

    fn from_request<'a>(parts: Request<B>, _state: &'a C) -> Self::Future<'a> {
        core::future::ready(Ok(parts))
    }
}

impl<C: 'static, B: 'static> FromRequest<B, C> for () {
    type Error = Infallible;

    type Future<'a> = core::future::Ready<Result<(), Self::Error>>;

    fn from_request<'a>(_parts: Request<B>, _state: &'a C) -> Self::Future<'a> {
        core::future::ready(Ok(()))
    }
}

macro_rules! from_request {
    ($first: ident) => {
        impl<B, C, $first> FromRequest<B, C> for ($first,)
        where
            C: 'static,
            B: 'static,
            $first: FromRequest<B, C>,
        {
            type Error = $first::Error;

            type Future<'a> = HBoxFuture<'a, Result<($first,), Self::Error>>;

            fn from_request<'a>(parts: Request<B>, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {
                    let ret = $first::from_request(parts, state).await?;
                    Ok((ret,))
                })
            }
        }

        impl<C, $first> FromRequestParts<C> for ($first,)
        where
            C: 'static,
            $first: FromRequestParts<C>,
            $first::Error: Into<Box<dyn core::error::Error + Send + Sync>>
        {
            type Error = FromRequestError;
            type Future<'a> = HBoxFuture<'a, Result<($first,), Self::Error>>;

            fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {
                    let ret = $first::from_request_parts(parts, state).await.map_err(|err|FromRequestError(err.into()))?;
                    Ok((ret,))
                })
            }
        }
    };
    ($first: ident, $($rest:ident),+) => {
        from_request!($($rest),+);

        impl<B, C, $first, $($rest),*> FromRequest<B, C> for ($($rest),+,$first)
        where
            C: 'static,
            B: 'static,
            $first: FromRequest<B, C>,
            $first::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
            $(
                $rest: FromRequestParts<C>,
                $rest::Error: Into<Box<dyn core::error::Error + Send + Sync>>

            ),+
        {
            type Error = FromRequestError;
            type Future<'a> = HBoxFuture<'a, Result<($($rest),+,$first), Self::Error>>;

            #[allow(non_snake_case, unused_parens)]
            fn from_request<'a>(req: Request<B>, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {

                    let (mut parts, data) = req.into_parts();

                    let ($($rest),+) = (
                        $(
                            $rest::from_request_parts(&mut parts, state).await.map_err(|err|FromRequestError(err.into()))?
                        ),+
                    );
                    let ret = $first::from_request(Request::from_parts(parts, data), state).await.map_err(|err|FromRequestError(err.into()))?;
                    Ok(($($rest),+,ret))
                })
            }
        }

        impl< C, $first, $($rest),*> FromRequestParts<C> for ($($rest),+,$first)
        where
            C: 'static,
            $first: FromRequestParts<C>,
            $first::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
            $(
                $rest: FromRequestParts<C>,
                $rest::Error: Into<Box<dyn core::error::Error + Send + Sync>>
            ),+
        {
            type Error = FromRequestError;
            type Future<'a> = HBoxFuture<'a, Result<($($rest),+,$first), Self::Error>>;

            #[allow(non_snake_case, unused_parens)]
            fn from_request_parts<'a>(parts: &'a mut Parts, state: &'a C) -> Self::Future<'a> {
                Box::pin(async move {


                    let ($($rest),+) = (
                        $(
                            $rest::from_request_parts(parts, state).await.map_err(|err|FromRequestError(err.into()))?
                        ),+
                    );
                    let ret = $first::from_request_parts(parts, state).await.map_err(|err|FromRequestError(err.into()))?;
                    Ok(($($rest),+,ret))
                })
            }
        }
    };
    ($($ty: ty => $method: ident),+) => {
        $(
            impl<C> FromRequestParts<C> for $ty {
                type Error = Infallible;
                type Future<'a> = core::future::Ready<Result<Self, Self::Error>> where Self: 'a, C: 'a;
                fn from_request_parts<'a>(parts: &'a mut Parts, _state: &'a C) -> Self::Future<'a> {
                    core::future::ready(Ok(parts.$method.clone()))
                }
            }

            impl<B: 'static, C, > FromRequest<B, C> for $ty {
                type Error = Infallible;
                type Future<'a> = core::future::Ready<Result<Self, Self::Error>> where Self: 'a, C: 'a;
                fn from_request<'a>(req: Request<B>, _state: &'a C) -> Self::Future<'a> {
                    core::future::ready(Ok(req.$method().clone()))
                }
            }
        )+
    };
}

from_request!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);

from_request!( Uri => uri, HeaderMap => headers);
