use std::net::SocketAddr;

use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper_util::rt::TokioIo;
use klaver::{Options, RuntimeError, pool::VmPoolOptions};
use reggie::Body;
use rquickjs::{Class, Ctx};
use tokio::net::TcpListener;
use wilbur_quick::{AppBuildCtx, AppBuilder, InitModule, InitPath, JsApp, Router};

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::Result<()> {
    let mut builder = AppBuilder::default();

    builder.add_init(|app: &mut AppBuildCtx<'_>| {
        app.add_module(wilbur_cookies::CookiesModule);
    });

    builder.add_init(InitModule(wilbur_cookies::CookiesModule));

    builder.add_init(InitPath("./wilbur-quick/examples/app.js"));

    // let vm = Options::default().search_path(".").build().await.unwrap();

    // klaver::async_with!(vm => |ctx| {

    //     let app = builder.build(ctx.clone()).await?;

    //     serve(ctx, app, SocketAddr::from(([127, 0, 0, 1], 3000))).await.unwrap();

    //     Ok(())
    // })
    // .await?;

    serve_multi(builder, SocketAddr::from(([127, 0, 0, 1], 3000))).await?;

    Ok(())
}

async fn serve<'js>(
    ctx: Ctx<'js>,
    app: Class<'js, JsApp<'js>>,
    addr: SocketAddr,
) -> color_eyre::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let app = app.clone();
        let cloned_ctx = ctx.clone();
        ctx.spawn(async move {
            // let svc = TowerToHyperService::new(router);
            let svc = hyper::service::service_fn(move |req| {
                let srv = app.clone();
                let ctx = cloned_ctx.clone();
                async move {
                    let req = req.map(|body: hyper::body::Incoming| {
                        Body::from_streaming(body.map_err(|err| reggie::Error::conn(err)))
                    });

                    srv.borrow().handle(ctx, req).await
                }
            });
            if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("server error: {}", err);
            }
        });
    }

    Ok(())
}

async fn serve_multi<'js>(builder: AppBuilder, addr: SocketAddr) -> color_eyre::Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let pool = klaver::pool::Manager::new(VmPoolOptions::from(
        klaver::Options::default().search_path("."),
    )?)?
    .init(move |ctx| {
        let builder = builder.clone();
        Box::pin(async move {
            //
            klaver::async_with!(ctx => |ctx| {
                let instance = builder.build(ctx.clone()).await?;

                ctx.globals().set("Wilbur", instance.clone())?;

                Ok(())
            })
            .await?;
            Ok(())
        })
    });

    let pool = klaver::pool::Pool::builder(pool).build()?;

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);

        let cloned_pool = pool.clone();

        tokio::spawn(async move {
            // let svc = TowerToHyperService::new(router);
            let pool = cloned_pool.clone();
            let svc = hyper::service::service_fn(move |req| {
                let pool = pool.clone();
                async move {
                    let conn = pool.get().await.unwrap();

                    let req = req.map(|body: hyper::body::Incoming| {
                        Body::from_streaming(body.map_err(|err| reggie::Error::conn(err)))
                    });

                    let resp = klaver::async_with!(conn => |ctx| {

                        let app = ctx.globals().get::<_, Class<JsApp>>("Wilbur")?;

                        let ret = app.borrow().handle(ctx, req).await?;

                        Ok(ret)
                    })
                    .await;

                    resp
                }
            });
            if let Err(err) = http1::Builder::new().serve_connection(io, svc).await {
                eprintln!("server error: {}", err);
            }
        });
    }

    Ok(())
}
