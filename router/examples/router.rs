use std::convert::Infallible;
use std::net::SocketAddr;

use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Bytes, service::Service};
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use tokio::net::TcpListener;

use http::{Request, Response};
use reggie::Body;
use router::{Error, MethodFilter, Middleware, Router, ServiceExt};

struct TestMiddleare;

impl<B, C, H> Middleware<B, C, H> for TestMiddleare {}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut router = Router::<(), Body>::new();

    let service = tower_util::service_fn(|req: Request<Body>| async move {
        Result::<_, Error>::Ok(Response::new(Body::from("Hello, World")))
    });

    router.route(MethodFilter::GET, "/", service.into_handle());

    // let ret = tokio::spawn(async move {
    //     let handle = router.get_mut("/", MethodFilter::GET).unwrap();
    //     handle.call(&(), Request::new(Body::empty())).await
    // })
    // .await
    // .unwrap();

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(addr).await?;
    let router = router.into_service(());
    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let router = router.clone();
        tokio::spawn(async move {
            // N.B. should use tower service_fn here, since it's required to be implemented tower Service trait before convert to hyper Service!

            // Convert it to hyper service
            let svc = TowerToHyperService::new(router);
            let svc = hyper::service::service_fn(move |req| {
                let srv = svc.clone();
                async move {
                    let req = req.map(|body: hyper::body::Incoming| {
                        reggie::Body::from_streaming(body.map_err(reggie::Error::conn))
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
