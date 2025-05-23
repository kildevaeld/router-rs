use std::{
    future::{Ready, ready},
    sync::Arc,
};

use http::{
    HeaderMap,
    header::{COOKIE, SET_COOKIE},
};
use parking_lot::RwLock;
use wilbur_container::Extensible;
use wilbur_core::{Error, FromRequest, FromRequestParts};

#[derive(Debug, Clone)]
pub struct CookieJar {
    pub(crate) inner: Arc<RwLock<cookie::CookieJar>>,
}

impl CookieJar {
    pub fn add<C>(&self, cookie: C)
    where
        C: Into<cookie::Cookie<'static>>,
    {
        self.inner.write().add(cookie)
    }

    pub fn get(&self, name: &str) -> Option<cookie::Cookie<'static>> {
        self.inner.read().get(name).cloned()
    }

    pub fn remove<C>(&self, cookie: C)
    where
        C: Into<cookie::Cookie<'static>>,
    {
        self.inner.write().remove(cookie)
    }

    #[cfg(feature = "signed")]
    pub fn signed<'a>(&self, key: &'a cookie::Key) -> crate::SignedJar<'a> {
        crate::SignedJar::new(self.clone(), key)
    }
}

impl CookieJar {
    pub(crate) fn from_headers(headers: &HeaderMap) -> CookieJar {
        let cookies = headers
            .get_all(COOKIE)
            .into_iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(';'))
            .filter_map(|cookie| cookie::Cookie::parse_encoded(cookie.to_owned()).ok());

        let mut jar = cookie::CookieJar::new();

        for cookie in cookies {
            jar.add_original(cookie);
        }

        CookieJar {
            inner: Arc::new(RwLock::new(jar)),
        }
    }

    pub fn apply(&self, headers: &mut HeaderMap) {
        let mut guard = self.inner.write();
        for cookie in guard.delta() {
            if let Ok(header_value) = cookie.encoded().to_string().parse() {
                headers.append(SET_COOKIE, header_value);
            }
        }
        guard.reset_delta();
    }
}

// #[async_trait]
// impl<C> FromRequestParts<C> for CookieJar {
//     type Rejection = &'static str;

//     async fn from_request_parts(parts: &mut Parts, _state: &C) -> Result<Self, Self::Rejection> {
//         parts
//             .extensions
//             .get::<CookieJar>()
//             .cloned()
//             .ok_or_else(|| "cookie jar not found")
//     }
// }

impl<C: Extensible> FromRequestParts<C> for CookieJar {
    type Future<'a>
        = Ready<Result<CookieJar, Error>>
    where
        C: 'a;
    fn from_request_parts<'a>(
        parts: &'a mut http::request::Parts,
        _state: &'a C,
    ) -> Self::Future<'a> {
        ready(
            parts
                .extensions
                .get::<CookieJar>()
                .cloned()
                .ok_or_else(|| Error::new("CookieJar modifier not registered")),
        )
    }
}

impl<B: 'static, C: Extensible> FromRequest<B, C> for CookieJar {
    type Future<'a>
        = Ready<Result<CookieJar, Error>>
    where
        C: 'a;

    fn from_request<'a>(parts: http::Request<B>, state: &'a C) -> Self::Future<'a> {
        let (mut parts, _) = parts.into_parts();
        Self::from_request_parts(&mut parts, state)
    }
}
