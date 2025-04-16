use std::{future::Future, sync::Arc};

use heather::HSend;
use http::{Request, Response};
use wilbur_cookies::{Cookie, CookieJar, Key};
use wilbur_core::{Modifier, Modify};

use crate::{SessionId, State};

pub struct SessionModifier(Arc<Key>);

impl Default for SessionModifier {
    fn default() -> Self {
        SessionModifier(Key::generate().into())
    }
}

impl<B: HSend, C> Modifier<B, C> for SessionModifier {
    type Modify = ModifySession;

    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        _state: &'a C,
    ) -> impl Future<Output = Self::Modify> + HSend + 'a {
        async move {
            let cookies = request.extensions().get::<CookieJar>().unwrap().clone();

            let id = match cookies.signed(&*self.0).get("sess_id") {
                Some(ret) => uuid::Uuid::parse_str(ret.value())
                    .map(|m| SessionId::new(m))
                    .unwrap_or_default(),
                None => SessionId::default(),
            };

            request.extensions_mut().insert(id.clone());

            ModifySession {
                cookies,
                session_id: id,
                key: self.0.clone(),
            }
        }
    }
}

pub struct ModifySession {
    cookies: CookieJar,
    session_id: SessionId,
    key: Arc<Key>,
}

impl<B: HSend, C> Modify<B, C> for ModifySession {
    fn modify<'a>(
        self,
        response: &'a mut Response<B>,
        _state: &'a C,
    ) -> impl std::future::Future<Output = ()> + 'a + HSend {
        async move {
            match self.session_id.state() {
                State::Remove(_) => {
                    self.cookies
                        .signed(&self.key)
                        .remove(Cookie::build("sess_id").path("/"));
                }
                State::Set(id) => {
                    self.cookies.signed(&self.key).add(
                        Cookie::build(("sess_id", id.to_string()))
                            .http_only(true)
                            .secure(true)
                            .path("/"),
                    );
                }
                _ => {}
            }

            self.cookies.apply(response.headers_mut());
        }
    }
}
