pub use body::Body;
use context::{Context, RouterContext};
use router::{MethodFilter, Middleware, Modifier, Routing};
use uhuh_container::modules::{Builder, Module};

mod body;
mod context;
mod error;

pub use error::Error;

use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;

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
        M: uhuh_container::modules::Module<RouterContext> + 'static,
        M::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        self.builder.add_module(module);
    }

    pub fn module<M>(mut self, module: M) -> Self
    where
        M: uhuh_container::modules::Module<RouterContext> + 'static,
        M::Error: Into<Box<dyn core::error::Error + Send + Sync>>,
    {
        self.builder.add_module(module);
        self
    }

    #[cfg(feature = "send")]
    pub async fn serve(self, addr: SocketAddr) -> Result<(), error::Error> {
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
                            Body::from_streaming(body.map_err(error::Error::from))
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

    #[cfg(not(feature = "send"))]
    pub async fn serve(self, addr: SocketAddr) -> Result<(), error::Error> {
        use body::Body;

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
                            Body::from_streaming(body.map_err(error::Error::from))
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
}

impl Routing<Context, Body> for App {
    type Handler = <RouterContext as Routing<Context, Body>>::Handler;

    fn modifier<M: router::Modifier<Body, Context> + 'static>(&mut self, modifier: M) {
        self.builder.add_module(ModifierModule(modifier));
    }

    fn route<T>(
        &mut self,
        method: router::MethodFilter,
        path: &str,
        handler: T,
    ) -> Result<(), router::Error>
    where
        T: router::Handler<Body, Context> + 'static,
    {
        self.builder.add_module(RouteModule {
            method,
            route: path.to_string(),
            handler,
        });

        Ok(())
    }

    fn middleware<M>(&mut self, middleware: M) -> Result<(), router::Error>
    where
        M: router::Middleware<Body, Context, Self::Handler> + 'static,
    {
        self.builder.add_module(MiddlewareModule(middleware));
        Ok(())
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
    T: router::Handler<Body, Context> + 'static,
{
    type Error = router::Error;

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
    M: Middleware<Body, Context, <RouterContext as Routing<Context, Body>>::Handler> + 'static,
{
    type Error = router::Error;

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
