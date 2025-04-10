pub use crate::body::Body;
use crate::context::{Context, RouterContext};
use wilbur_container::modules::{Builder, Module};
use wilbur_core::{Handler, Middleware, Modifier};
#[cfg(feature = "hyper")]
use wilbur_routing::service::RouterService;
use wilbur_routing::{MethodFilter, RouteError, Routing};

#[cfg(feature = "serve")]
use {
    crate::error::Error, http_body_util::BodyExt, hyper::server::conn::http1,
    hyper::service::Service, hyper_util::rt::TokioIo, std::net::SocketAddr,
    tokio::net::TcpListener,
};

use std::convert::Infallible;

pub struct App {
    builder: Builder<RouterContext>,
}

impl App {
    pub fn new() -> App {
        App {
            builder: Builder::new(),
        }
    }

    pub fn add_module<M>(&mut self, module: M)
    where
        M: wilbur_container::modules::Module<RouterContext> + 'static,
        M::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        self.builder.add_module(module);
    }

    pub fn module<M>(mut self, module: M) -> Self
    where
        M: wilbur_container::modules::Module<RouterContext> + 'static,
        M::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        self.builder.add_module(module);
        self
    }

    #[cfg(all(feature = "send", feature = "serve"))]
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Error> {
        let (router, context) = self.builder.build(RouterContext::new()).await.unwrap();

        let listener = TcpListener::bind(addr).await?;
        let router = router.into_service(context);
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let router = router.clone();
            tokio::task::spawn(async move {
                // let svc = TowerToHyperService::new(router);
                let svc = router;
                let svc = hyper::service::service_fn(move |req| {
                    let srv = svc.clone();
                    async move {
                        let req = req.map(|body: hyper::body::Incoming| {
                            Body::from_streaming(body.map_err(Error::from))
                        });

                        srv.call(req).await
                    }
                });
                if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                    eprintln!("server error: {}", err);
                }
            });
        }

        Ok(())
    }

    #[cfg(all(not(feature = "send"), feature = "serve"))]
    pub async fn serve(self, addr: SocketAddr) -> Result<(), Error> {
        let (router, context) = self.builder.build(RouterContext::new()).await.unwrap();

        let listener = TcpListener::bind(addr).await?;
        let router = router.into_service(context);
        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);
            let router = router.clone();
            tokio::task::spawn_local(async move {
                // let svc = TowerToHyperService::new(router);
                let svc = router;
                let svc = hyper::service::service_fn(move |req| {
                    let srv = svc.clone();
                    async move {
                        let req = req.map(|body: hyper::body::Incoming| {
                            Body::from_streaming(body.map_err(Error::from))
                        });

                        srv.call(req).await
                    }
                });
                if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                    eprintln!("server error: {}", err);
                }
            });
        }
    }

    #[cfg(feature = "hyper")]
    pub async fn into_service(self) -> RouterService<Body, Context> {
        let (router, context) = self.builder.build(RouterContext::new()).await.unwrap();
        router.into_service(context)
    }
}

impl Routing<Body, Context> for App {
    type Handler = <RouterContext as Routing<Body, Context>>::Handler;

    fn modifier<M: Modifier<Body, Context> + 'static>(&mut self, modifier: M) {
        self.builder.add_module(ModifierModule(modifier));
    }

    fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<Body, Context> + 'static,
    {
        self.builder.add_module(RouteModule {
            method,
            route: path.to_string(),
            handler,
        });

        Ok(())
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), RouteError>
    where
        M: Middleware<Body, Context, Self::Handler> + 'static,
    {
        self.builder.add_module(MiddlewareModule(middleware));
        Ok(())
    }

    fn merge(&mut self, router: Self) -> Result<(), RouteError> {
        todo!()
    }

    fn mount<T: Into<Self>>(&mut self, path: &str, router: T) -> Result<(), RouteError> {
        todo!()
    }
}

struct ModifierModule<M>(M);

impl<M> Module<RouterContext> for ModifierModule<M>
where
    M: Modifier<Body, Context> + 'static,
{
    type Error = Infallible;

    fn build<'a>(
        self,
        ctx: &'a mut RouterContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            ctx.modifier(self.0);
            Ok(())
        }
    }
}

struct RouteModule<T> {
    method: MethodFilter,
    route: String,
    handler: T,
}

impl<T> Module<RouterContext> for RouteModule<T>
where
    T: Handler<Body, Context> + 'static,
{
    type Error = RouteError;

    fn build<'a>(
        self,
        ctx: &'a mut RouterContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            ctx.route(self.method, &self.route, self.handler)?;
            Ok(())
        }
    }
}

struct MiddlewareModule<M>(M);

impl<M> Module<RouterContext> for MiddlewareModule<M>
where
    M: Middleware<Body, Context, <RouterContext as Routing<Body, Context>>::Handler> + 'static,
{
    type Error = RouteError;

    fn build<'a>(
        self,
        ctx: &'a mut RouterContext,
    ) -> impl Future<Output = Result<(), Self::Error>> + heather::HSend + 'a {
        async move {
            ctx.middleware(self.0)?;
            Ok(())
        }
    }
}

pub fn app() -> App {
    App::new()
}
