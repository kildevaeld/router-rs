use std::{sync::Arc, task::Poll};

use crate::{
    handler::{BoxHandler, Handler, box_handler},
    middleware::{BoxMiddleware, Middleware, box_middleware},
};
use heather::BoxFuture;
use http::{Method, Request, Response};

pub use crate::error::Error;
use crate::traits::{MaybeSend, MaybeSendSync};

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MethodFilter: u8 {
       const GET = 1 << 0;
       const POST = 1 << 1;
       const PUT = 1 << 2;
       const PATCH = 1 << 3;
       const DELETE = 1 << 4;
    }
}

impl MethodFilter {
    pub fn any() -> MethodFilter {
        MethodFilter::all()
    }
}

impl From<Method> for MethodFilter {
    fn from(value: Method) -> Self {
        match value {
            Method::GET => MethodFilter::GET,
            Method::POST => MethodFilter::POST,
            Method::PATCH => MethodFilter::PATCH,
            Method::PUT => MethodFilter::PUT,
            Method::DELETE => MethodFilter::DELETE,
            _ => todo!(),
        }
    }
}

struct Pair<B, C> {
    method: MethodFilter,
    handle: BoxHandler<B, C>,
}

pub struct RouteHandler<C, B> {
    handlers: Vec<Pair<B, C>>,
    name: Option<String>,
}

pub struct Router<C, B> {
    tree: routing::Router<RouteHandler<C, B>>,
    middlewares: Vec<BoxMiddleware<B, C, BoxHandler<B, C>>>,
}

impl<C: MaybeSendSync + 'static, B: MaybeSend + 'static> Router<C, B> {
    pub fn new() -> Router<C, B> {
        Router {
            tree: routing::Router::new(),
            middlewares: Default::default(),
        }
    }

    pub fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), Error>
    where
        T: Handler<B, C> + 'static,
    {
        if let Some(route) = self.tree.get_route_mut(path) {
            if route
                .handlers
                .iter()
                .find(|m| m.method.contains(method))
                .is_some()
            {
                panic!("Route already defined")
            }

            route.handlers.push(Pair {
                method,
                handle: box_handler(handler),
            });
        } else {
            self.tree.register(
                path,
                RouteHandler {
                    handlers: vec![Pair {
                        method: method,
                        handle: box_handler(handler),
                    }],
                    name: None,
                },
            )?;
        }

        Ok(())
    }

    pub fn middleware<M>(&mut self, middleware: M) -> Result<(), Error>
    where
        M: Middleware<B, C, BoxHandler<B, C>> + 'static,
    {
        self.middlewares.push(box_middleware(middleware));
        Ok(())
    }

    pub fn get(&self, path: &str, method: MethodFilter) -> Option<&BoxHandler<B, C>> {
        self.tree.match_path(path, &mut ()).and_then(|m| {
            m.handlers.iter().find_map(|m| {
                if m.method.contains(method) {
                    Some(&m.handle)
                } else {
                    None
                }
            })
        })
    }

    pub fn get_mut(&mut self, path: &str, method: MethodFilter) -> Option<&mut BoxHandler<B, C>> {
        self.tree.match_path_mut(path, &mut ()).and_then(|m| {
            m.handlers.iter_mut().find_map(|m| {
                if m.method.contains(method) {
                    Some(&mut m.handle)
                } else {
                    None
                }
            })
        })
    }

    pub fn into_service(self, context: C) -> RouterService<C, B> {
        RouterService {
            router: self.into(),
            context,
        }
    }
}

pub struct RouterService<C, B> {
    router: Arc<Router<C, B>>,
    context: C,
}

impl<C, B> tower::Service<Request<B>> for RouterService<C, B>
where
    B: MaybeSend + 'static,
    C: Clone + MaybeSendSync + 'static,
{
    type Response = Response<B>;

    type Error = Error;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let router = self.router.clone();
        let context = self.context.clone();
        Box::pin(async move {
            //
            let Some(handle) = router.get(req.uri().path(), req.method().clone().into()) else {
                todo!()
            };

            handle.call(&context, req).await
        })
    }
}

impl<C, B> Clone for RouterService<C, B>
where
    C: Clone,
{
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
            context: self.context.clone(),
        }
    }
}
