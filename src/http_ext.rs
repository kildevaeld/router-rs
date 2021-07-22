use super::{Params, Router};
use http::{Method, Request};

#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

pub struct HttpRoute<H> {
    pub method: Option<Method>,
    pub handle: H,
}

#[derive(Default, Clone)]
pub struct RouteParams {
    inner: BTreeMap<String, String>,
}

impl RouteParams {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner.get(key).map(|m| m.as_str())
    }
}

impl<'a> Params<'a> for RouteParams {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str) {
        self.inner.insert(key.to_string(), value.to_string());
    }
}

pub trait RouterExt<'a, H: 'a> {
    type Iter: Iterator<Item = &'a H> + 'a;
    fn match_request<'c, B>(&'c self, req: &'a Request<B>) -> Option<Self::Iter>
    where
        'c: 'a;
}

impl<'a, H: 'a> RouterExt<'a, H> for Router<HttpRoute<H>> {
    // type Iter = Box<dyn Iterator<Item = &'a H> + 'a>;
    type Iter = MatchRequestIter<'a, H>;
    fn match_request<'c, B>(&'c self, req: &'a Request<B>) -> Option<Self::Iter>
    where
        'c: 'a,
    {
        let mut params = BTreeMap::default();

        let found = match self.find(req.uri().path(), &mut params) {
            Some(found) => found,
            None => return None,
        };

        Some(MatchRequestIter {
            iter: found,
            next: if found.is_empty() { None } else { Some(0) },
            method: req.method(),
        })
    }
}

pub struct MatchRequestIter<'a, H: 'a> {
    iter: &'a Vec<HttpRoute<H>>,
    next: Option<usize>,
    method: &'a Method,
}

impl<'a, H: 'a> Iterator for MatchRequestIter<'a, H> {
    type Item = &'a H;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.next.is_none() {
                return None;
            }
            let idx = self.next.unwrap();

            let next = &self.iter[idx];

            self.next = if idx + 1 == self.iter.len() {
                None
            } else {
                Some(idx + 1)
            };

            if let Some(method) = &next.method {
                if method != &self.method {
                    continue;
                }
            }

            return Some(&next.handle);
        }
    }
}
