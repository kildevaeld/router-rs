use core::{fmt, str::FromStr};
use std::{boxed::Box, format};

use alloc::{vec, vec::Vec};

use http::Method;

use crate::{AsSegments, Params, PathRouter};

#[derive(Debug)]
pub struct RouteError {
    inner: Box<dyn std::error::Error + Send + Sync>,
}

impl RouteError {
    pub fn new<T: Into<Box<dyn std::error::Error + Send + Sync>>>(error: T) -> RouteError {
        RouteError {
            inner: error.into(),
        }
    }
}

impl fmt::Display for RouteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RouteError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        Some(&*self.inner)
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
    pub struct MethodFilter: u8 {
       const GET = 1 << 0;
       const POST = 1 << 1;
       const PUT = 1 << 2;
       const PATCH = 1 << 3;
       const DELETE = 1 << 4;
       const HEAD = 1 << 5;
    }
}

impl MethodFilter {
    pub fn any() -> MethodFilter {
        MethodFilter::all()
    }
}

impl FromStr for MethodFilter {
    type Err = RouteError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ret = match s {
            "GET" => MethodFilter::GET,
            "POST" => MethodFilter::POST,
            "PATCH" => MethodFilter::PATCH,
            "PUT" => MethodFilter::PUT,
            "DELETE" => MethodFilter::DELETE,
            "HEAD" => MethodFilter::HEAD,
            _ => return Err(RouteError::new(format!("Unknown method: '{s}'"))),
        };

        Ok(ret)
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
            Method::HEAD => MethodFilter::HEAD,
            _ => todo!(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Entry<H> {
    pub method: MethodFilter,
    pub handler: H,
}

#[derive(Debug, Clone)]
pub struct Route<H> {
    pub entries: Vec<Entry<H>>,
}

#[derive(Debug, Clone)]
pub struct Router<H> {
    inner: PathRouter<Route<H>>,
}

impl<H> Router<H> {
    pub fn new() -> Router<H> {
        Router {
            inner: PathRouter::new(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&crate::Segments<'_>, &Route<H>)> {
        self.inner.iter()
    }

    pub fn map<T, U>(self, mapper: T) -> Router<U>
    where
        T: Fn(H) -> U + Copy,
    {
        Router {
            inner: self.inner.map(move |route| Route {
                entries: route
                    .entries
                    .into_iter()
                    .map(move |m| Entry {
                        handler: mapper(m.handler),
                        method: m.method,
                    })
                    .collect(),
            }),
        }
    }

    pub fn mount<'a, S: AsSegments<'a>>(
        &mut self,
        path: S,
        router: Router<H>,
    ) -> Result<(), RouteError> {
        self.inner.mount(path, router.inner);
        Ok(())
    }

    pub fn merge(&mut self, router: Router<H>) -> Result<(), RouteError> {
        self.inner.merge(router.inner);
        Ok(())
    }

    pub fn route(
        &mut self,
        method: MethodFilter,
        path: &str,
        handler: H,
    ) -> Result<(), RouteError> {
        if let Some(route) = self.inner.get_route_mut(path) {
            if route
                .entries
                .iter()
                .find(|m| m.method.contains(method))
                .is_some()
            {
                return Err(RouteError {
                    inner: Box::from("Route already defined"),
                });
            }

            route.entries.push(Entry { method, handler });
        } else {
            self.inner
                .register(
                    path,
                    Route {
                        entries: vec![Entry {
                            method: method,
                            handler,
                        }],
                    },
                )
                .map_err(|err| RouteError {
                    inner: Box::new(err),
                })?;
        }

        Ok(())
    }

    pub fn match_route<P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> Option<&H> {
        self.inner.match_path(path, params).and_then(|m| {
            m.entries.iter().find_map(|m| {
                if m.method.contains(method) {
                    Some(&m.handler)
                } else {
                    None
                }
            })
        })
    }

    pub fn match_routes<'a, P: Params>(
        &self,
        path: &str,
        method: MethodFilter,
        params: &mut P,
    ) -> RouteMatchIter<'_, H> {
        RouteMatchIter {
            inner: self
                .inner
                .match_path(path, params)
                .map(|m| m.entries.iter()),
            method,
        }
    }
}

pub struct RouteMatchIter<'a, H> {
    inner: Option<core::slice::Iter<'a, Entry<H>>>,
    method: MethodFilter,
}

impl<'a, H> Iterator for RouteMatchIter<'a, H> {
    type Item = &'a H;
    fn next(&mut self) -> Option<Self::Item> {
        let Some(iter) = self.inner.as_mut() else {
            return None;
        };

        loop {
            let next = iter.next()?;
            if next.method.contains(self.method) {
                return Some(&next.handler);
            }
        }
    }
}
