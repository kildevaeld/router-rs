use routing::router::MethodFilter;
use wilbur_core::{Handler, Middleware, Modifier};

use crate::error::RouteError;

pub trait Routing<B, C> {
    type Handler: Handler<B, C>;

    fn modifier<M: Modifier<B, C> + 'static>(&mut self, modifier: M);

    fn route<T>(&mut self, method: MethodFilter, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<B, C> + 'static;

    fn middleware<M>(&mut self, middleware: M) -> Result<(), RouteError>
    where
        M: Middleware<B, C, Self::Handler> + 'static;

    fn mount(&mut self, path: &str, router: Self) -> Result<(), RouteError>;

    fn merge(&mut self, router: Self) -> Result<(), RouteError>;
}

pub trait RoutingExt<B, C>: Routing<B, C> {
    fn get<T>(&mut self, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<B, C> + 'static,
    {
        self.route(MethodFilter::GET, path, handler)
    }

    fn post<T>(&mut self, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<B, C> + 'static,
    {
        self.route(MethodFilter::POST, path, handler)
    }

    fn patch<T>(&mut self, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<B, C> + 'static,
    {
        self.route(MethodFilter::PATCH, path, handler)
    }

    fn put<T>(&mut self, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<B, C> + 'static,
    {
        self.route(MethodFilter::PUT, path, handler)
    }

    fn delete<T>(&mut self, path: &str, handler: T) -> Result<(), RouteError>
    where
        T: Handler<B, C> + 'static,
    {
        self.route(MethodFilter::DELETE, path, handler)
    }
}

impl<B, C, T> RoutingExt<B, C> for T where T: Routing<B, C> {}
