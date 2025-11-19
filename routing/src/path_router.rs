use super::{AsSegments, Params, Segment, Segments};
use crate::arena::{Arena, Id};
use crate::matcher::into_segments;
use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

#[derive(Debug, Clone)]
pub struct Route<'a, H> {
    pub segments: Segments<'a>,
    pub handlers: Option<H>,
}

impl<'a, H> Route<'a, H> {
    pub fn to_owned(self) -> Route<'a, H> {
        Route {
            segments: self.segments.to_owned(),
            handlers: self.handlers,
        }
    }

    pub fn path(&self) -> String {
        self.segments.to_string()
    }

    pub fn map<F, U>(self, func: F) -> Route<'a, U>
    where
        F: Fn(H) -> U,
    {
        Route {
            segments: self.segments,
            handlers: self.handlers.map(func),
        }
    }
}

#[derive(Debug, Clone)]
struct Named<H> {
    name: String,
    handle: H,
}

#[derive(Debug, Clone)]
struct Node<H> {
    constants: BTreeMap<String, Id>,
    handle: Option<H>,
    catchall: Option<Named<Id>>,
    wildcard: Option<Named<Id>>,
    segments: Option<Segments<'static>>,
}

impl<H> Default for Node<H> {
    fn default() -> Node<H> {
        Node {
            constants: Default::default(),
            handle: Default::default(),
            catchall: None,
            wildcard: None,
            segments: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathRouter<H> {
    arena: Arena<Node<H>>,
    root: Id,
}

impl<H> PathRouter<H> {
    pub fn new() -> PathRouter<H> {
        let mut arena = Arena::default();
        let root = arena.alloc(Node::default());
        PathRouter { arena, root }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Segments<'_>, &H)> {
        self.arena
            .iter()
            .filter_map(|m| match (&m.segments, &m.handle) {
                (Some(s), Some(h)) => Some((s, h)),
                _ => None,
            })
    }

    pub fn register<'a, S: AsSegments<'a> + 'a>(
        &mut self,
        path: S,
        handle: H,
    ) -> Result<&mut Self, S::Error> {
        let mut current = self.root;

        let segments = path
            .as_segments()?
            .map(|m| m.to_owned())
            .collect::<Vec<_>>();

        'path: for segment in &segments {
            //
            match segment {
                Segment::Constant(path) => {
                    if let Some(node) = self.arena[current].constants.get(path.as_ref()) {
                        current = *node;
                        continue 'path;
                    }

                    let node = self.arena.alloc(Node::default());
                    self.arena[current].constants.insert(path.to_string(), node);
                    current = node;
                }
                Segment::Parameter(param) => {
                    if let Some(wildcard) = &self.arena[current].wildcard {
                        // TODO: Check if names is the same
                        current = wildcard.handle;
                        continue 'path;
                    } else {
                        let node = self.arena.alloc(Node::default());
                        self.arena[current].wildcard = Some(Named {
                            name: param.to_string(),
                            handle: node,
                        });
                        current = node;
                        continue 'path;
                    };
                }
                Segment::Star(star) => {
                    if let Some(star) = &self.arena[current].catchall {
                        current = star.handle;
                    } else {
                        let node = self.arena.alloc(Node::default());
                        self.arena[current].catchall = Some(Named {
                            name: star.to_string(),
                            handle: node,
                        });
                        current = node;
                        continue 'path;
                    }
                }
            };
        }

        self.arena[current].segments = Some(Segments(segments));
        self.arena[current].handle = Some(handle);

        Ok(self)
    }

    pub fn get_route<'a, S: AsSegments<'a>>(&self, path: S) -> Option<&H> {
        let node = self.get_route_inner(path)?;
        self.arena[node].handle.as_ref()
    }

    pub fn get_route_mut<'a, S: AsSegments<'a>>(&mut self, path: S) -> Option<&mut H> {
        let node = self.get_route_inner(path)?;
        self.arena[node].handle.as_mut()
    }

    fn get_route_inner<'a, S: AsSegments<'a>>(&self, path: S) -> Option<Id> {
        let mut current = self.root;

        let segments = path
            .as_segments()
            .ok()?
            .map(|m| m.to_owned())
            .collect::<Vec<_>>();

        'path: for segment in &segments {
            //
            match segment {
                Segment::Constant(path) => {
                    if let Some(node) = self.arena[current].constants.get(path.as_ref()) {
                        current = *node;
                        continue 'path;
                    }

                    return None;
                }
                Segment::Parameter(_) => {
                    //
                    if let Some(wildcard) = &self.arena[current].wildcard {
                        // TODO: Check if names is the same
                        current = wildcard.handle;
                        continue 'path;
                    } else {
                        return None;
                    };
                }
                Segment::Star(_) => {
                    //
                    if let Some(star) = &self.arena[current].catchall {
                        current = star.handle;
                    } else {
                        return None;
                    }
                }
            };
        }

        Some(current)
    }

    pub fn clear(&mut self) {
        self.arena = Arena::default();
        let root = self.arena.alloc(Node::default());
        self.root = root;
    }

    pub fn merge<'a>(&mut self, router: PathRouter<H>) {
        for (path, handler) in router {
            self.register(path, handler).expect("register");
        }
    }

    pub fn mount<'a, 'b, S: AsSegments<'a>>(
        &mut self,
        path: S,
        router: PathRouter<H>,
    ) -> Result<(), S::Error> {
        let mount = path.as_segments()?.collect::<Vec<_>>();
        for (path, handler) in router {
            let mut mount = mount.clone();
            mount.extend(path);
            self.register(mount, handler).expect("register");
        }

        Ok(())
    }

    fn match_path_inner<'b, 'c, P: Params>(&self, path: &str, params: &'c mut P) -> Option<Id> {
        let mut current_node = self.root;
        let mut catch_all = self.arena[current_node].catchall.as_ref();

        let segments = into_segments(path);

        let mut start = 0;

        for seg in segments {
            start = seg.start;
            if let Some(catch) = &self.arena[current_node].catchall {
                catch_all = Some(catch);
            }

            if let Some(constant) = self.arena[current_node].constants.get(&path[seg.clone()]) {
                current_node = *constant;
            } else if let Some(wildcard) = &self.arena[current_node].wildcard {
                params.set((&wildcard.name).into(), path[seg].into());
                current_node = wildcard.handle;
            } else if let Some(catch) = catch_all {
                let star = &path[seg.start..];
                params.set((&catch.name).into(), star.into());
                return Some(catch.handle);
            } else {
                return None;
            }
        }

        if let Some(_) = self.arena[current_node].handle.as_ref() {
            return Some(current_node);
        } else if let Some(catch) = catch_all {
            let star = &path[start..];
            params.set((&catch.name).into(), star.into());
            return Some(catch.handle);
        } else {
            return None;
        }
    }

    pub fn match_path<'a, 'c, P: Params>(&'a self, path: &str, params: &'c mut P) -> Option<&'a H> {
        let found = self.match_path_inner(path, params)?;
        self.arena[found].handle.as_ref()
    }

    pub fn match_path_mut<'a, 'c, P: Params>(
        &'a mut self,
        path: &str,
        params: &'c mut P,
    ) -> Option<&'a mut H> {
        let found = self.match_path_inner(path, params)?;
        self.arena[found].handle.as_mut()
    }

    pub fn map<F, V>(self, mut mapper: F) -> PathRouter<V>
    where
        F: FnMut(H, Option<&Segments<'_>>) -> V,
    {
        PathRouter {
            arena: self.arena.map(move |m| {
                let segments = m.segments;
                let handle = m.handle.map(|h| mapper(h, segments.as_ref()));
                Node {
                    constants: m.constants,
                    handle,
                    catchall: m.catchall,
                    wildcard: m.wildcard,
                    segments,
                }
            }),
            root: self.root,
        }
    }
}

impl<'a, H> IntoIterator for PathRouter<H> {
    type IntoIter = IntoIter<H>;
    type Item = (Segments<'static>, H);
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.arena.into_iter())
    }
}

pub struct IntoIter<H>(alloc::vec::IntoIter<Node<H>>);

impl<H> Iterator for IntoIter<H> {
    type Item = (Segments<'static>, H);
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some(next) = self.0.next() else {
                return None;
            };

            match (next.segments, next.handle) {
                (Some(segments), Some(handle)) => return Some((segments, handle)),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod test {
    pub use super::*;
    use alloc::collections::BTreeMap;

    #[test]
    fn test() {
        let mut router = PathRouter::new();

        router.register(&[], "root").unwrap();

        assert_eq!(
            router.match_path("", &mut BTreeMap::default()),
            Some(&"root")
        );
        assert_eq!(
            router.match_path("/", &mut BTreeMap::default()),
            Some(&"root")
        );
    }

    #[test]
    fn test_router() {
        let mut router = PathRouter::new();

        router
            .register(&[Segment::Constant("path".into())], "/path")
            .unwrap()
            .register(
                &[
                    Segment::Constant("path".into()),
                    Segment::Parameter("id".into()),
                ],
                "/path/:id",
            )
            .unwrap()
            .register(
                &[
                    Segment::Constant("statics".into()),
                    Segment::Star("filename".into()),
                ],
                "/statics/*filename",
            )
            .unwrap();

        assert_eq!(
            router.match_path("path", &mut BTreeMap::default()),
            Some(&"/path")
        );
        assert_eq!(
            router.match_path("/path", &mut BTreeMap::default()),
            Some(&"/path")
        );
        let mut m = BTreeMap::default();
        assert_eq!(router.match_path("/path/10", &mut m), Some(&"/path/:id"));
        assert_eq!(m.get("id"), Some(&"10".into()));

        assert_eq!(
            router.match_path("/statics/filename.png", &mut BTreeMap::default()),
            Some(&"/statics/*filename")
        );
    }

    // #[test]
    // fn test_extend() {
    //     let mut router1 = Router::new();

    //     router1
    //         .register(&[Segment::constant("statics")], "statics")
    //         .expect("statics");

    //     router1
    //         .register(
    //             &[Segment::constant("statics"), Segment::constant("something")],
    //             "",
    //         )
    //         .expect("something");

    //     let mut router2 = Router::new();

    //     router2
    //         .register(&[Segment::constant("statics")], "statics2")
    //         .expect("statics");

    //     router1.extend(router2);

    //     // assert_eq!(
    //     //     router1.find("/statics", &mut BTreeMap::default()),
    //     //     Some(&"statics", "statics2"])
    //     // );
    // }

    // #[test]
    // fn test_mount() {
    //     let mut router1 = Router::new();

    //     router1
    //         .register(&[Segment::constant("statics")], "statics")
    //         .expect("statics");

    //     router1
    //         .register(
    //             &[Segment::constant("statics"), Segment::constant("something")],
    //             "",
    //         )
    //         .expect("something");

    //     let mut router2 = Router::new();

    //     router2
    //         .register(&[Segment::constant("statics")], "statics2")
    //         .expect("statics");

    //     router1.mount("/api", router2).expect("mount");

    //     assert_eq!(
    //         router1.match_path("/api/statics", &mut BTreeMap::default()),
    //         Some(&"statics2")
    //     );
    // }
}
