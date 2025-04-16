use std::sync::Arc;

use heather::HBoxFuture;
use rquickjs::{CatchResultExt, Class, Ctx, Function, Value};
use rquickjs_util::RuntimeError;
use wilbur_container::modules::{BuildContext, Module};

use crate::{
    JsApp,
    context::{JsBuildContext, JsRouteContext},
    router::Router,
};

pub trait ModuleInit<'js> {
    fn call<'a>(
        self: Box<Self>,
        ctx: &'a mut JsBuildContext<'js>,
    ) -> HBoxFuture<'a, Result<(), RuntimeError>>;
}

struct Wrapper<T>(T);

impl<'js, T> ModuleInit<'js> for Wrapper<T>
where
    T: Module<JsBuildContext<'js>> + 'js,
    T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
{
    fn call<'a>(
        self: Box<Self>,
        ctx: &'a mut JsBuildContext<'js>,
    ) -> HBoxFuture<'a, Result<(), RuntimeError>> {
        Box::pin(async move {
            self.0
                .build(ctx)
                .await
                .map_err(|err| RuntimeError::Custom(err.into()))?;
            Ok(())
        })
    }
}

pub struct AppBuildCtx<'js> {
    modules: Vec<Box<dyn ModuleInit<'js> + 'js>>,
    builder: Option<rquickjs_modules::Builder>,
}

impl<'js> AppBuildCtx<'js> {
    pub fn add_module<T: Module<JsBuildContext<'js>> + 'js>(&mut self, module: T)
    where
        T: 'js,
        T::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        self.modules.push(Box::new(Wrapper(module)));
    }

    pub fn register_module<T: rquickjs_modules::ModuleInfo>(&mut self) -> &mut Self {
        self.mutate(|ctx| ctx.module::<T>());
        self
    }

    pub fn register_globals<T: rquickjs_modules::GlobalInfo>(&mut self) -> &mut Self {
        self.mutate(|ctx| ctx.global::<T>());
        self
    }

    pub async fn build(
        self,
        ctx: Ctx<'js>,
    ) -> rquickjs::Result<(Class<'js, Router<'js>>, JsRouteContext)> {
        let mut context = JsBuildContext::new(ctx)?;

        for module in self.modules {
            module.call(&mut context).await;
        }

        Ok(context.build().await.unwrap())
    }

    fn mutate<T>(&mut self, func: T)
    where
        T: FnOnce(rquickjs_modules::Builder) -> rquickjs_modules::Builder,
    {
        let builder = self.builder.take().unwrap();

        let builder = func(builder);

        self.builder = Some(builder);
    }
}

pub trait Init {
    fn init<'js>(&self, ctx: &mut AppBuildCtx<'js>);
}

impl<T> Init for T
where
    for<'a> T: Fn(&mut AppBuildCtx<'a>),
{
    fn init<'js>(&self, app: &mut AppBuildCtx<'js>) {
        (self)(app)
    }
}

#[derive(Default, Clone)]
pub struct AppBuilder {
    inits: Vec<Arc<dyn Init + Send + Sync>>,
}

impl AppBuilder {
    pub fn add_init<I: Init + Send + Sync + 'static>(&mut self, init: I) {
        self.inits.push(Arc::from(init));
    }

    pub async fn build<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<Class<'js, JsApp<'js>>> {
        let mut app = AppBuildCtx {
            modules: Default::default(),
            builder: Some(rquickjs_modules::Builder::new()),
        };
        for init in &self.inits {
            init.init(&mut app);
        }

        let (router, context) = app.build(ctx.clone()).await?;

        let router = router.borrow_mut().build();

        let instance = Class::instance(ctx.clone(), JsApp { router, context })?;

        Ok(instance)
    }
}

pub struct InitModule<T>(pub T);

impl<T> Init for InitModule<T>
where
    T: Clone + 'static,
    for<'js> <T as Module<JsBuildContext<'js>>>::Error:
        Into<Box<dyn core::error::Error + Send + Sync>>,
    for<'js> T: Module<JsBuildContext<'js>>,
{
    fn init<'js>(&self, app: &mut AppBuildCtx<'js>) {
        app.add_module(self.0.clone());
    }
}

pub struct InitPath<S>(pub S);

impl<S> Init for InitPath<S>
where
    S: Into<Vec<u8>> + Clone + 'static,
{
    fn init<'js>(&self, app: &mut AppBuildCtx<'js>) {
        app.add_module(InitPathModule(self.0.clone()));
    }
}

pub struct InitPathModule<S>(S);

impl<'js, S> Module<JsBuildContext<'js>> for InitPathModule<S>
where
    S: Into<Vec<u8>> + 'js,
{
    type Error = RuntimeError;

    fn build<'a>(
        self,
        ctx: &'a mut JsBuildContext<'js>,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            let module = rquickjs::Module::import(&*ctx, self.0)
                .catch(&*ctx)?
                .into_future::<rquickjs::Object>()
                .await
                .catch(&*ctx)?;

            let func = module.get::<_, Function>("default").catch(&*ctx)?;

            let output = func.call::<_, Value>((ctx.router.clone(),)).catch(&*ctx)?;

            if let Some(promise) = output.as_promise() {
                promise.clone().into_future::<()>().await.catch(&*ctx)?;
            }

            Ok(())
        }
    }
}
