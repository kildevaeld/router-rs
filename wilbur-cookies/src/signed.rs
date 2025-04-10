use cookie::{Cookie, Key};

use crate::CookieJar;

pub struct SignedJar<'a> {
    parent: CookieJar,
    key: &'a Key,
}

impl<'a> SignedJar<'a> {
    pub fn new(parent: CookieJar, key: &'a Key) -> SignedJar<'a> {
        SignedJar { parent, key }
    }
    pub fn verify(&self, cookie: Cookie<'static>) -> Option<Cookie<'static>> {
        self.parent.inner.read().signed(self.key).verify(cookie)
    }

    pub fn add<C: Into<Cookie<'static>>>(&mut self, cookie: C) {
        self.parent.inner.write().signed_mut(self.key).add(cookie);
    }

    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        self.parent.inner.write().signed(self.key).get(name)
    }

    pub fn remove<C: Into<Cookie<'static>>>(&mut self, cookie: C) {
        self.parent
            .inner
            .write()
            .signed_mut(self.key)
            .remove(cookie)
    }
}
