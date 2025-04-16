use std::any::Any;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::{borrow::Cow, future::Future};

use futures::future::LocalBoxFuture;
use klaver::RuntimeError;
use rquickjs::Ctx;
use rquickjs::{FromIteratorJs, JsLifetime, Value, class::Trace};
use rquickjs_util::StringRef;

pub trait Augmentation<C, T> {
    #[allow(unused)]
    fn typings<'a>(
        &'a self,
        core: &'a C,
    ) -> impl Future<Output = Result<Option<Cow<'static, str>>, klaver::RuntimeError>> + 'a {
        async move { Ok(None) }
    }

    fn apply_augmentation<'a, 'js: 'a>(
        &'a self,
        ctx: Ctx<'js>,
        obj: &'a mut Augment<'js>,
        item: &'a mut T,
        wilbur: &'a C,
    ) -> impl Future<Output = Result<(), klaver::RuntimeError>> + 'a;
}

pub trait DynAugmentation<C>: Send + Sync {
    fn typings<'a>(
        &'a self,
        core: &'a C,
    ) -> LocalBoxFuture<'a, Result<Option<Cow<'static, str>>, klaver::RuntimeError>>;

    fn apply_augment<'a, 'js: 'a>(
        &'a self,
        ctx: Ctx<'js>,
        obj: &'a mut Augment<'js>,
        item: &'a mut dyn Any,
        wilbur: &'a C,
    ) -> LocalBoxFuture<'a, Result<(), RuntimeError>>;
}

pub struct AugmentBox<T, V>(T, PhantomData<V>);

impl<T, V> AugmentBox<T, V> {
    pub fn new(augment: T) -> AugmentBox<T, V> {
        AugmentBox(augment, PhantomData)
    }
}

unsafe impl<T: Send, V> Send for AugmentBox<T, V> {}

unsafe impl<T: Sync, V> Sync for AugmentBox<T, V> {}

impl<C, T, V> DynAugmentation<C> for AugmentBox<T, V>
where
    T: Augmentation<C, V> + Send + Sync,
    V: 'static,
{
    fn typings<'a>(
        &'a self,
        core: &'a C,
    ) -> LocalBoxFuture<'a, Result<Option<Cow<'static, str>>, klaver::RuntimeError>> {
        Box::pin(async move { self.0.typings(core).await })
    }

    fn apply_augment<'a, 'js: 'a>(
        &'a self,
        ctx: Ctx<'js>,
        obj: &'a mut Augment<'js>,
        item: &'a mut dyn Any,
        wilbur: &'a C,
    ) -> LocalBoxFuture<'a, Result<(), RuntimeError>> {
        Box::pin(async move {
            //
            let Some(item) = item.downcast_mut::<V>() else {
                return Err(RuntimeError::Message(Some(
                    "Could not downcast item".to_string(),
                )));
            };

            self.0.apply_augmentation(ctx, obj, item, wilbur).await?;

            Ok(())
        })
    }
}

#[rquickjs::class]
#[derive(Default)]
pub struct Augment<'js> {
    factories: HashMap<String, Box<dyn FnOnce(Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>>>>,
    values: HashMap<String, rquickjs::Value<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for Augment<'js> {
    type Changed<'to> = Augment<'to>;
}

impl<'js> Trace<'js> for Augment<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {
        self.values.trace(tracer)
    }
}

impl<'js> Augment<'js> {
    pub fn register<T>(&mut self, name: impl ToString, factory: T) -> &mut Self
    where
        T: FnOnce(Ctx<'js>) -> rquickjs::Result<rquickjs::Value<'js>> + 'static,
    {
        self.factories.insert(name.to_string(), Box::new(factory));
        self
    }
}

#[rquickjs::methods]
impl<'js> Augment<'js> {
    pub fn get(
        &mut self,
        ctx: Ctx<'js>,
        name: StringRef<'js>,
    ) -> rquickjs::Result<rquickjs::Value<'js>> {
        if let Some(factory) = self.factories.remove(name.as_str()) {
            let value = (factory)(ctx)?;
            self.values.insert(name.to_string(), value.clone());
            return Ok(value);
        }

        if let Some(value) = self.values.get(name.as_str()).cloned() {
            Ok(value)
        } else {
            Ok(Value::new_null(ctx))
        }
    }

    pub fn list(&self, ctx: Ctx<'js>) -> rquickjs::Result<rquickjs::Array<'js>> {
        let mut keys = self.values.keys().cloned().collect::<Vec<_>>();

        keys.extend(self.factories.keys().cloned());

        rquickjs::Array::from_iter_js(&ctx, keys)
    }
}
