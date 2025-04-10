use rquickjs::{atom::PredefinedAtom, class::Trace, JsLifetime};
use rquickjs_util::Val;
use uhuh_core::vaerdi::Value;

use crate::Session;

#[derive(JsLifetime)]
#[rquickjs::class(rename = "Session")]
pub(crate) struct JsSession {
    pub session: Session,
}

impl<'js> Trace<'js> for JsSession {
    fn trace<'a>(&self, _tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl JsSession {
    fn get(&self, name: String) -> rquickjs::Result<Val> {
        let Some(cookie) = self.session.get(&name) else {
            return Ok(Val(Value::Null));
        };

        Ok(Val(cookie.clone()))
    }

    fn set(&mut self, name: String, value: Val) -> rquickjs::Result<()> {
        self.session.set(&name, value.0);
        Ok(())
    }

    fn remove(&mut self, name: String) -> rquickjs::Result<()> {
        self.session.remove(&name);
        Ok(())
    }

    async fn delete(&mut self) -> rquickjs::Result<()> {
        self.session.delete().await;
        Ok(())
    }

    async fn save(&mut self) -> rquickjs::Result<()> {
        self.session.save().await;
        Ok(())
    }

    #[qjs(rename = "regenerateId")]
    async fn generate_id(&mut self) -> rquickjs::Result<()> {
        self.session.regenerate_id().await;
        Ok(())
    }

    #[qjs(rename = PredefinedAtom::SymbolToStringTag)]
    fn string_tag() -> rquickjs::Result<String> {
        Ok(String::from("Session"))
    }
}
