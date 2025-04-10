use crate::CookieJar;
use heather::HSend;
use http::{Request, Response};
use std::future::Future;
use wilbur_core::{Modifier, Modify};

pub struct CookiesJarModifier;

impl<B, C> Modifier<B, C> for CookiesJarModifier {
    type Modify = ModifyCookie;

    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        _state: &'a C,
    ) -> impl Future<Output = Self::Modify> + HSend + 'a {
        async move {
            let cookie_jar = CookieJar::from_headers(request.headers());
            request.extensions_mut().insert(cookie_jar.clone());
            ModifyCookie { cookie_jar }
        }
    }
}

pub struct ModifyCookie {
    cookie_jar: CookieJar,
}

impl<B, C> Modify<B, C> for ModifyCookie {
    fn modify<'a>(
        self,
        response: &'a mut Response<B>,
        _state: &'a C,
    ) -> impl std::future::Future<Output = ()> + 'a + HSend {
        async move {
            self.cookie_jar.apply(response.headers_mut());
        }
    }
}
