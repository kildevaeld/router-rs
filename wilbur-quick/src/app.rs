use std::{any::TypeId, collections::BTreeMap, sync::Arc};

use futures::future::BoxFuture;
use klaver::{RuntimeError, Vm};
use rquickjs::Class;
use rquickjs_modules::Environ;

use crate::{Init, JsApp, augment::DynAugmentation, context::JsRouteContext};

pub struct App {
    pub inits: Arc<[Arc<dyn Init + Send + Sync>]>,
    pub env: Environ,
    pub augmentations: BTreeMap<TypeId, Vec<Box<dyn DynAugmentation<JsRouteContext>>>>,
}

impl App {
    #[cfg(all(feature = "hyper", feature = "send"))]
    pub async fn into_service(self) -> Result<AppService, klaver::RuntimeError> {
        Ok(AppService {
            pool: self.create_pool().await?,
        })
    }

    #[cfg(all(feature = "hyper", not(feature = "send")))]
    pub async fn into_service(self) -> Result<AppService, klaver::RuntimeError> {
        Ok(AppService {
            vm: self.create_vm().await?.into(),
        })
    }

    #[cfg(all(feature = "hyper", feature = "send"))]
    pub async fn serve(self, addr: std::net::SocketAddr) -> Result<(), RuntimeError> {
        use http_body_util::BodyExt;
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let pool = self.create_pool().await?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|err| RuntimeError::Custom(Box::new(err)))?;

        loop {
            let (stream, _) = listener
                .accept()
                .await
                .map_err(|err| RuntimeError::Custom(Box::new(err)))?;
            let io = TokioIo::new(stream);

            let cloned_pool = pool.clone();

            tokio::spawn(async move {
                let pool = cloned_pool.clone();
                let svc = hyper::service::service_fn(move |req| {
                    let pool = pool.clone();
                    async move {
                        let conn = pool.get().await.unwrap();

                        let req = req.map(|body: hyper::body::Incoming| {
                            reggie::Body::from_streaming(body.map_err(reggie::Error::conn))
                        });

                        klaver::async_with!(conn => |ctx| {

                            let app = ctx.globals().get::<_, Class<JsApp>>("Wilbur")?;

                            let ret = app.borrow().handle(ctx, req).await?;

                            Ok(ret)
                        })
                        .await
                    }
                });
                if let Err(err) = hyper::server::conn::http1::Builder::new()
                    .serve_connection(io, svc)
                    .await
                {
                    eprintln!("server error: {}", err);
                }
            });
        }

        Ok(())
    }

    #[cfg(all(feature = "hyper", not(feature = "send")))]
    pub async fn serve(self, addr: std::net::SocketAddr) -> Result<(), RuntimeError> {
        use http_body_util::BodyExt;
        use hyper_util::rt::TokioIo;
        use tokio::net::TcpListener;

        let vm = self.create_vm().await?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|err| RuntimeError::Custom(Box::new(err)))?;

        klaver::async_with!(vm => |ctx| {

            let app = ctx.globals().get::<_, Class<JsApp>>("Wilbur")?;

            loop {
                let (stream, _) = listener
                    .accept()
                    .await
                    .map_err(|err| RuntimeError::Custom(Box::new(err)))?;
                let io = TokioIo::new(stream);

                let app = app.clone();
                let cloned_ctx = ctx.clone();

                ctx.spawn(async move {
                    // let svc = TowerToHyperService::new(router);
                    let app = app.clone();
                    let ctx = cloned_ctx.clone();
                    let svc = hyper::service::service_fn(move |req| {
                        let app = app.clone();
                        let ctx = ctx.clone();
                        async move {

                            let req = req.map(|body: hyper::body::Incoming| {
                                reggie::Body::from_streaming(
                                    body.map_err(|err| reggie::Error::conn(err)),
                                )
                            });

                            app.borrow().handle(ctx, req).await

                        }
                    });
                    if let Err(err) = hyper::server::conn::http1::Builder::new()
                        .serve_connection(io, svc)
                        .await
                    {
                        eprintln!("server error: {}", err);
                    }
                });
            }

            Ok(())


        })
        .await?;

        Ok(())
    }

    #[cfg(feature = "send")]
    async fn create_pool(self) -> Result<klaver::pool::Pool, klaver::RuntimeError> {
        let inits = self.inits.clone();

        let manager = klaver::pool::Manager::new(klaver::pool::VmPoolOptions {
            max_stack_size: None,
            memory_limit: None,
            modules: self.env,
            worker_thread: false,
        })?
        .init(move |ctx| {
            let inits = inits.clone();
            Box::pin(async move {
                //
                let inits = inits.clone();
                klaver::async_with!(ctx => |ctx| {
                    let instance = crate::build(&inits, ctx.clone()).await?;

                    ctx.globals().set("Wilbur", instance.clone())?;

                    Ok(())
                })
                .await?;
                Ok(())
            })
        });
        let pool = klaver::pool::Pool::builder(manager)
            .build()
            .map_err(|err| klaver::RuntimeError::Custom(Box::new(err)))?;

        Ok(pool)
    }

    #[cfg(not(feature = "send"))]
    async fn create_vm(self) -> Result<klaver::Vm, klaver::RuntimeError> {
        let inits = self.inits;

        let vm = Vm::new_with(&self.env, None, None).await?;

        klaver::async_with!(vm => |ctx| {
            let instance = crate::build(&inits, ctx.clone()).await?;

            ctx.globals().set("Wilbur", instance.clone())?;

            Ok(())
        })
        .await?;

        Ok(vm)
    }
}

#[cfg(all(feature = "hyper", feature = "send"))]
pub struct AppService {
    pool: klaver::pool::Pool,
}

#[cfg(all(feature = "hyper", feature = "send"))]
impl hyper::service::Service<http::Request<reggie::Body>> for AppService {
    type Response = http::Response<reggie::Body>;

    type Error = klaver::RuntimeError;

    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&self, req: http::Request<reggie::Body>) -> Self::Future {
        let pool = self.pool.clone();
        Box::pin(async move {
            let vm = pool
                .get()
                .await
                .map_err(|err| klaver::RuntimeError::Custom(Box::new(err)))?;
            klaver::async_with!(vm => |ctx| {
                let app = ctx.globals().get::<_, Class<JsApp>>("Wilbur")?;
                let app_b = app.borrow();

                let resp  =app_b.handle(ctx, req).await?;

                Ok(resp)
            })
            .await
        })
    }
}

#[cfg(all(feature = "hyper", not(feature = "send")))]
pub struct AppService {
    vm: heather::Hrc<klaver::Vm>,
}

#[cfg(all(feature = "hyper", not(feature = "send")))]
impl hyper::service::Service<http::Request<reggie::Body>> for AppService {
    type Response = http::Response<reggie::Body>;

    type Error = klaver::RuntimeError;

    type Future = futures::future::LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn call(&self, req: http::Request<reggie::Body>) -> Self::Future {
        let vm = self.vm.clone();
        Box::pin(async move {
            klaver::async_with!(vm => |ctx| {
                let app = ctx.globals().get::<_, Class<JsApp>>("Wilbur")?;
                let app_b = app.borrow();

                let resp = app_b.handle(ctx, req).await?;

                Ok(resp)
            })
            .await
        })
    }
}
