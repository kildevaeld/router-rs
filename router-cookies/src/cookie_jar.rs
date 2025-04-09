use std::sync::Arc;

use heather::{Hrc, Lock};
use http::{
    HeaderMap,
    header::{COOKIE, SET_COOKIE},
    request::Parts,
};
use parking_lot::RwLock;

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
