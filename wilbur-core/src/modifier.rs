use heather::{HBoxFuture, HSend, HSendSync, Hrc};
use http::{Request, Response};
use std::future::Future;

use crate::{handler::Handler, into_response::IntoResponse, middleware::Middleware};

pub trait Modifier<B, C>: HSendSync {
    type Modify: Modify<B, C>;
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> impl Future<Output = Self::Modify> + 'a + HSend;
}

pub trait Modify<B, C>: HSend {
    fn modify<'a>(
        self,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> impl Future<Output = ()> + 'a + HSend;
}

pub trait DynModifier<B, C>: HSendSync {
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> HBoxFuture<'a, BoxModify<B, C>>;
}

pub trait DynModify<B, C>: HSend {
    fn modify<'a>(
        self: Box<Self>,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> HBoxFuture<'a, ()>;
}

struct ModifyBox<T>(T);

impl<T, B: HSend, C: HSendSync> DynModify<B, C> for ModifyBox<T>
where
    T: Modify<B, C> + HSend + 'static,
{
    fn modify<'a>(
        self: Box<Self>,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> HBoxFuture<'a, ()> {
        Box::pin(async move { self.0.modify(response, state).await })
    }
}

#[derive(Debug, Clone, Copy)]
struct ModifierBox<T>(T);

impl<T, B, C> DynModifier<B, C> for ModifierBox<T>
where
    T: Modifier<B, C> + HSendSync,
    T::Modify: HSend + 'static,
    C: HSendSync,
    B: HSend,
{
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> HBoxFuture<'a, BoxModify<B, C>> {
        Box::pin(async move {
            Box::new(ModifyBox(self.0.before(request, state).await)) as BoxModify<B, C>
        })
    }
}

pub fn modifier_box<T, B, C>(modifier: T) -> BoxModifier<B, C>
where
    T: Modifier<B, C> + HSendSync + 'static,
    T::Modify: HSend + 'static,
    C: HSendSync,
    B: HSend + 'static,
{
    Hrc::new(ModifierBox(modifier))
}

pub type BoxModifier<B, C> = Hrc<dyn DynModifier<B, C>>;
pub type BoxModify<B, C> = Box<dyn DynModify<B, C>>;

pub struct ModifierMiddleware<B, C> {
    modifiers: Hrc<[BoxModifier<B, C>]>,
}

impl<B, C> ModifierMiddleware<B, C> {
    pub fn new(modifiers: impl Into<Hrc<[BoxModifier<B, C>]>>) -> ModifierMiddleware<B, C> {
        ModifierMiddleware {
            modifiers: modifiers.into(),
        }
    }
}

impl<B, C, H> Middleware<B, C, H> for ModifierMiddleware<B, C>
where
    B: HSend + 'static,
    C: HSendSync + Clone + 'static,
    H: Handler<B, C> + HSendSync + Clone + 'static,
    H::Response: IntoResponse<B>,
{
    type Handler = ModifierMiddlewareHandler<B, C, H>;

    fn wrap(&self, handler: H) -> Self::Handler {
        ModifierMiddlewareHandler {
            modifiers: self.modifiers.clone(),
            handler,
        }
    }
}

pub struct ModifierMiddlewareHandler<B, C, H> {
    modifiers: Hrc<[BoxModifier<B, C>]>,
    handler: H,
}

impl<B, C, H> Handler<B, C> for ModifierMiddlewareHandler<B, C, H>
where
    B: HSend + 'static,
    C: HSendSync + Clone + 'static,
    H: Handler<B, C> + HSendSync + Clone + 'static,
    H::Response: IntoResponse<B>,
{
    type Error = H::Error;
    type Response = Response<B>;

    type Future<'a> = HBoxFuture<'a, Result<Self::Response, Self::Error>>;

    fn call<'a>(&'a self, context: &'a C, mut req: Request<B>) -> Self::Future<'a> {
        let service = self.handler.clone();
        let modifiers = self.modifiers.clone();

        Box::pin(async move {
            let mut mods = Vec::with_capacity(modifiers.len());

            for modifier in modifiers.iter() {
                mods.push(modifier.before(&mut req, context).await);
            }

            let mut res = service.call(context, req).await?.into_response();

            for modifier in mods {
                modifier.modify(&mut res, context).await;
            }

            Ok(res)
        })
    }
}

impl<B: HSend, C: HSendSync> Modifier<B, C> for Hrc<[BoxModifier<B, C>]> {
    type Modify = Vec<BoxModify<B, C>>;
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> impl Future<Output = Self::Modify> + 'a + HSend {
        async move {
            let mut modifiers = Vec::with_capacity(self.len());
            for m in self.iter() {
                modifiers.push(m.before(request, state).await);
            }

            modifiers
        }
    }
}

impl<B: HSend, C: HSendSync> Modify<B, C> for Vec<BoxModify<B, C>> {
    fn modify<'a>(
        self,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> impl Future<Output = ()> + 'a + HSend {
        async move {
            for m in self {
                m.modify(response, state).await;
            }
        }
    }
}

pub struct ModifierList<B, C>(Hrc<[BoxModifier<B, C>]>);

impl<B, C> Clone for ModifierList<B, C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<B, C> From<Hrc<[BoxModifier<B, C>]>> for ModifierList<B, C> {
    fn from(value: Hrc<[BoxModifier<B, C>]>) -> Self {
        ModifierList(value)
    }
}

impl<B, C> From<Vec<BoxModifier<B, C>>> for ModifierList<B, C> {
    fn from(value: Vec<BoxModifier<B, C>>) -> Self {
        ModifierList(value.into())
    }
}

impl<B, C> From<ModifierList<B, C>> for Vec<BoxModifier<B, C>> {
    fn from(value: ModifierList<B, C>) -> Self {
        value.0.iter().cloned().collect()
    }
}

impl<B: HSend, C: HSendSync> Modifier<B, C> for ModifierList<B, C> {
    type Modify = Vec<BoxModify<B, C>>;

    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> impl Future<Output = Self::Modify> + 'a + HSend {
        self.0.before(request, state)
    }
}
