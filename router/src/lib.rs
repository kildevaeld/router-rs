// use handler::{BoxHandler, Handler, box_handler};
// use http::{Method, Request, Response};
// use tower::Service;

mod error;
mod handler;
mod into_response;
mod middleware;
mod router;
mod service_ext;
mod traits;

pub use self::error::Error;
pub use self::into_response::IntoResponse;
pub use self::middleware::Middleware;
pub use self::router::{MethodFilter, Router};
pub use self::service_ext::ServiceExt;
// pub use self::service_ext::ServiceExt;
// use self::traits::{MaybeSend, MaybeSendSync};
// pub use error::Error;

// bitflags::bitflags! {
//     #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
//     pub struct MethodFilter: u8 {
//        const GET = 1 << 0;
//        const POST = 1 << 1;
//        const PUT = 1 << 2;
//        const PATCH = 1 << 3;
//        const DELETE = 1 << 4;
//     }
// }

// impl MethodFilter {
//     pub fn any() -> MethodFilter {
//         MethodFilter::all()
//     }
// }

// struct Pair<B, C> {
//     method: MethodFilter,
//     handle: BoxHandler<B, C>,
// }

// pub struct RouteHandler<C, B> {
//     handlers: Vec<Pair<B, C>>,
//     name: Option<String>,
// }

// pub struct Router<C, B> {
//     tree: routing::Router<RouteHandler<C, B>>,
// }

// impl<C: MaybeSendSync + 'static, B: MaybeSend + 'static> Router<C, B> {
//     pub fn new() -> Router<C, B> {
//         Router {
//             tree: routing::Router::new(),
//         }
//     }

//     pub fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), Error>
//     where
//         T: Handler<B, C> + 'static,
//     {
//         if let Some(route) = self.tree.get_route_mut(path) {
//             if route
//                 .handlers
//                 .iter()
//                 .find(|m| m.method.contains(method))
//                 .is_some()
//             {
//                 panic!("Route already defined")
//             }

//             route.handlers.push(Pair {
//                 method,
//                 handle: box_handler(handler),
//             });
//         } else {
//             self.tree.register(
//                 path,
//                 RouteHandler {
//                     handlers: vec![Pair {
//                         method: method,
//                         handle: box_handler(handler),
//                     }],
//                     name: None,
//                 },
//             )?;
//         }

//         Ok(())
//     }

//     pub fn get(&self, path: &str, method: MethodFilter) -> Option<&BoxHandler<B, C>> {
//         self.tree.match_path(path, &mut ()).and_then(|m| {
//             m.handlers.iter().find_map(|m| {
//                 if m.method.contains(method) {
//                     Some(&m.handle)
//                 } else {
//                     None
//                 }
//             })
//         })
//     }

//     pub fn get_mut(&mut self, path: &str, method: MethodFilter) -> Option<&mut BoxHandler<B, C>> {
//         self.tree.match_path_mut(path, &mut ()).and_then(|m| {
//             m.handlers.iter_mut().find_map(|m| {
//                 if m.method.contains(method) {
//                     Some(&mut m.handle)
//                 } else {
//                     None
//                 }
//             })
//         })
//     }
// }
