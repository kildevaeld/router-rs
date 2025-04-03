use crate::{Error, Handler, IntoResponse, Middleware, traits::*};
use heather::Hrc;
use http::{Request, Response};
use std::{convert::Infallible, future::Future, pin::Pin, sync::Arc};
#[cfg(feature = "tower")]
use tower::{Layer, Service};

pub trait Modifier<B, C>: MaybeSendSync {
    type Modify: Modify<B, C>;
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> impl Future<Output = Self::Modify> + 'a + MaybeSend;
}

pub trait Modify<B, C>: MaybeSend {
    fn modify<'a>(
        self,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> impl Future<Output = ()> + 'a + MaybeSend;
}

pub trait DynModifier<B, C>: MaybeSendSync {
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> BoxFuture<'a, BoxModify<B, C>>;
}

pub trait DynModify<B, C>: MaybeSend {
    fn modify<'a>(
        self: Box<Self>,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> BoxFuture<'a, ()>;
}

struct ModifyBox<T>(T);

impl<T, B: MaybeSend, C: MaybeSendSync> DynModify<B, C> for ModifyBox<T>
where
    T: Modify<B, C> + MaybeSend + 'static,
{
    fn modify<'a>(
        self: Box<Self>,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.0.modify(response, state).await })
    }
}

#[derive(Debug, Clone, Copy)]
struct ModifierBox<T>(T);

impl<T, B, C> DynModifier<B, C> for ModifierBox<T>
where
    T: Modifier<B, C> + MaybeSendSync,
    T::Modify: MaybeSend + 'static,
    C: MaybeSendSync,
    B: MaybeSend,
{
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> BoxFuture<'a, BoxModify<B, C>> {
        Box::pin(async move {
            Box::new(ModifyBox(self.0.before(request, state).await)) as BoxModify<B, C>
        })
    }
}

pub fn modifier_box<T, B, C>(modifier: T) -> BoxModifier<B, C>
where
    T: Modifier<B, C> + MaybeSendSync + 'static,
    T::Modify: Send + 'static,
    C: MaybeSendSync,
    B: MaybeSend + 'static,
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
    B: MaybeSend + 'static,
    C: MaybeSendSync + Clone + 'static,
    H: Handler<B, C> + MaybeSendSync + Clone + 'static,
    H::Response: IntoResponse<B>,
{
    type Handle = ModifierMiddlewareHandler<B, C, H>;

    fn wrap(&self, handler: H) -> Self::Handle {
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
    B: MaybeSend + 'static,
    C: MaybeSendSync + Clone + 'static,
    H: Handler<B, C> + MaybeSendSync + Clone + 'static,
    H::Response: IntoResponse<B>,
{
    type Response = Response<B>;

    type Future<'a> = BoxFuture<'a, Result<Self::Response, Error>>;

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

#[cfg(feature = "tower")]
#[derive(Clone)]
pub struct ModifierLayer<B, C> {
    modifiers: Arc<[BoxModifier<B, C>]>,
    state: C,
}

#[cfg(feature = "tower")]
impl<B, C> ModifierLayer<B, C> {
    pub fn new(modifiers: Vec<BoxModifier<B, C>>, state: C) -> ModifierLayer<B, C> {
        ModifierLayer {
            modifiers: modifiers.into(),
            state,
        }
    }
}

#[cfg(feature = "tower")]
impl<T, B: MaybeSend, C: 'static + MaybeSendSync + Clone> Layer<T> for ModifierLayer<B, C> {
    type Service = ModifierLayerService<T, B, C>;
    fn layer(&self, inner: T) -> Self::Service {
        ModifierLayerService {
            service: inner,
            state: self.state.clone(),
            modifiers: self.modifiers.clone(),
        }
    }
}

#[cfg(feature = "tower")]
#[derive(Clone)]
pub struct ModifierLayerService<T, B, C> {
    service: T,
    state: C,
    modifiers: Arc<[BoxModifier<B, C>]>,
}

#[cfg(feature = "tower")]
impl<T, B, C: 'static + Clone + MaybeSendSync> Service<Request<B>> for ModifierLayerService<T, B, C>
where
    T: Service<Request<B>, Error = Infallible> + Clone + Send + 'static,
    T::Response: IntoResponse<B>,
    T::Future: MaybeSend,
    B: MaybeSend + 'static,
{
    type Error = Infallible;
    type Response = Response<B>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        let mut service = self.service.clone();
        let modifiers = self.modifiers.clone();
        let state = self.state.clone();

        Box::pin(async move {
            let mut mods = Vec::with_capacity(modifiers.len());

            req.extensions_mut().insert(state.clone());

            for modifier in modifiers.iter() {
                mods.push(modifier.before(&mut req, &state).await);
            }

            let mut res = service.call(req).await?.into_response();

            for modifier in mods {
                modifier.modify(&mut res, &state).await;
            }

            Ok(res)
        })
    }
}

impl<B: MaybeSend, C: MaybeSendSync> Modifier<B, C> for Hrc<[BoxModifier<B, C>]> {
    type Modify = Vec<BoxModify<B, C>>;
    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> impl Future<Output = Self::Modify> + 'a + MaybeSend {
        async move {
            let mut modifiers = Vec::with_capacity(self.len());
            for m in self.iter() {
                modifiers.push(m.before(request, state).await);
            }

            modifiers
        }
    }
}

impl<B: MaybeSend, C: MaybeSendSync> Modify<B, C> for Vec<BoxModify<B, C>> {
    fn modify<'a>(
        self,
        response: &'a mut Response<B>,
        state: &'a C,
    ) -> impl Future<Output = ()> + 'a + MaybeSend {
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

impl<B: MaybeSend, C: MaybeSendSync> Modifier<B, C> for ModifierList<B, C> {
    type Modify = Vec<BoxModify<B, C>>;

    fn before<'a>(
        &'a self,
        request: &'a mut Request<B>,
        state: &'a C,
    ) -> impl Future<Output = Self::Modify> + 'a + MaybeSend {
        self.0.before(request, state)
    }
}
