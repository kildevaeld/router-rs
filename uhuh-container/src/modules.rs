use crate::{Extensible, ExtensibleMut};
use alloc::{boxed::Box, vec::Vec};
use heather::{BoxFuture, HBoxError, HSend, HSendSync};

pub trait BuildContext: ExtensibleMut {
    type Context: Extensible;
    type Output;
    type Error;

    fn build(self) -> impl Future<Output = Result<Self::Output, Self::Error>> + HSend;
}

pub trait Module<C: BuildContext>: HSendSync {
    type Error;
    fn build<'a>(
        self,
        ctx: &'a mut C,
    ) -> impl Future<Output = Result<(), Self::Error>> + HSend + 'a;
}

pub trait DynModule<C: BuildContext>: HSendSync {
    fn build<'a>(self: Box<Self>, ctx: &'a mut C) -> BoxFuture<'a, Result<(), HBoxError<'static>>>;
}

pub type BoxModule<C> = Box<dyn DynModule<C>>;

impl<C> Module<C> for BoxModule<C>
where
    C: BuildContext,
{
    type Error = HBoxError<'static>;

    fn build<'a>(
        self,
        ctx: &'a mut C,
    ) -> impl Future<Output = Result<(), Self::Error>> + HSend + 'a {
        async move { self.build(ctx).await }
    }
}

pub struct ModuleBox<T>(T);

impl<T> ModuleBox<T> {
    pub fn new<C>(module: T) -> Box<dyn DynModule<C>>
    where
        C: BuildContext,
        T: Module<C> + 'static,
        T::Error: Into<HBoxError<'static>> + 'static,
    {
        Box::new(ModuleBox(module))
    }
}

impl<C, T> DynModule<C> for ModuleBox<T>
where
    C: BuildContext,
    T: Module<C> + 'static,
    T::Error: Into<HBoxError<'static>> + 'static,
{
    fn build<'a>(self: Box<Self>, ctx: &'a mut C) -> BoxFuture<'a, Result<(), HBoxError<'static>>> {
        Box::pin(async move { self.0.build(ctx).await.map_err(Into::into) })
    }
}

pub struct Builder<C: BuildContext> {
    modules: Vec<BoxModule<C>>,
}

impl<C: BuildContext> Builder<C>
where
    C::Error: From<HBoxError<'static>>,
{
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn add_module<M>(&mut self, module: M)
    where
        M: Module<C> + 'static,
        M::Error: Into<HBoxError<'static>> + 'static,
    {
        self.modules.push(ModuleBox::new(module));
    }

    pub fn with_module<M>(mut self, module: M) -> Self
    where
        M: Module<C> + 'static,
        M::Error: Into<HBoxError<'static>> + 'static,
    {
        self.modules.push(ModuleBox::new(module));
        self
    }

    pub async fn build(self, mut ctx: C) -> Result<C::Output, C::Error> {
        for module in self.modules {
            module.build(&mut ctx).await.map_err(|e| {
                // Handle the error conversion if necessary
                e.into()
            })?;
        }
        ctx.build().await
    }
}
