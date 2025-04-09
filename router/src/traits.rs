pub use heather::{HBoxFuture, HSend as MaybeSend, HSendSync as MaybeSendSync};
use routing::router::MethodFilter;

use crate::{Error, Handler, Middleware, Modifier};

pub trait Routing<C, B> {
    type Handler: Handler<B, C>;

    fn modifier<M: Modifier<B, C> + 'static>(&mut self, modifier: M);

    fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), Error>
    where
        T: Handler<B, C> + 'static;

    fn middleware<M>(&mut self, middleware: M) -> Result<(), Error>
    where
        M: Middleware<B, C, Self::Handler> + 'static;
}
