use cookie::Cookie;
use rquickjs::{class::Trace, Class, JsLifetime};

use crate::CookieJar;

#[derive(JsLifetime)]
#[rquickjs::class]
pub(crate) struct JsCookieJar {
    pub cookies: CookieJar,
}

impl<'js> Trace<'js> for JsCookieJar {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl JsCookieJar {
    fn get(&self, name: String) -> rquickjs::Result<JsCookie> {
        let Some(cookie) = self.cookies.get(&name) else {
            todo!()
        };

        Ok(JsCookie { cookie })
    }

    fn add(&self, cookie: Class<'_, JsCookie>) -> rquickjs::Result<()> {
        self.cookies.add(cookie.borrow().cookie.clone());
        Ok(())
    }
}

#[derive(JsLifetime)]
#[rquickjs::class]
struct JsCookie {
    cookie: Cookie<'static>,
}

impl<'js> Trace<'js> for JsCookie {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl JsCookie {
    #[qjs(get, rename = "name")]
    pub fn name(&self) -> rquickjs::Result<String> {
        Ok(self.cookie.name().to_string())
    }

    #[qjs(get, rename = "value")]
    pub fn value(&self) -> rquickjs::Result<String> {
        Ok(self.cookie.value().to_string())
    }

    #[qjs(get, rename = "httpOnly")]
    pub fn http_only(&self) -> rquickjs::Result<Option<bool>> {
        Ok(self.cookie.http_only())
    }
}
