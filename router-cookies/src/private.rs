use cookie::{Cookie, Key};

use crate::CookieJar;

pub struct PrivateJar {
    parent: CookieJar,
    key: Key,
}

impl PrivateJar {
    pub fn new(parent: CookieJar, key: Key) -> PrivateJar {
        PrivateJar { parent, key }
    }

    pub fn verify(&self, cookie: Cookie<'static>) -> Option<Cookie<'static>> {
        self.parent.inner.read().private(&self.key).verify(cookie)
    }

    pub fn add<C: Into<Cookie<'static>>>(&mut self, cookie: C) {
        self.parent.inner.write().private_mut(&self.key).add(cookie);
    }

    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.parent.inner.write().private(&self.key).get(name)
    }

    pub fn remove<C: Into<Cookie<'static>>>(&mut self, cookie: C) {
        self.parent
            .inner
            .write()
            .private_mut(&self.key)
            .remove(cookie)
    }
}
