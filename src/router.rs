use super::{AsSegments, Params, Segment, Segments};
use crate::parser::into_segments;
use id_arena::{Arena, Id};
use std::{
    collections::HashMap,
    string::{String, ToString},
    vec::Vec,
};

#[derive(Debug, Clone)]
pub struct Route<'a, H> {
    pub segments: Segments<'a>,
    pub handlers: Vec<H>,
}

impl<'a, H> Route<'a, H> {
    pub fn to_owned(self) -> Route<'a, H> {
        Route {
            segments: self.segments.to_static(),
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
            handlers: self.handlers.into_iter().map(func).collect(),
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
    constants: HashMap<String, Id<Node<H>>>,
    handle: Option<Vec<H>>,
    catchall: Option<Named<Id<Node<H>>>>,
    wildcard: Option<Named<Id<Node<H>>>>,
    segments: Option<Segments<'static>>,
}

impl<H> Default for Node<H> {
    fn default() -> Node<H> {
        Node {
            constants: HashMap::default(),
            handle: None,
            catchall: None,
            wildcard: None,
            segments: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Router<H> {
    arena: Arena<Node<H>>,
    root: Id<Node<H>>,
}

impl<H> Router<H> {
    pub fn new() -> Router<H> {
        let mut arena = Arena::new();
        let root = arena.alloc(Node::default());
        Router { arena, root }
    }

    pub fn routes<'a>(&'a self) -> impl Iterator<Item = &'a Segments<'static>> {
        self.arena
            .iter()
            .filter(|m| m.1.segments.is_some())
            .map(|m| m.1.segments.as_ref().unwrap())
    }

    pub fn register<'a, S: AsSegments<'a> + 'a>(
        &mut self,
        path: S,
        handle: H,
    ) -> Result<&mut Self, S::Error> {
        let mut current = self.root;

        let segments = path
            .as_segments()?
            .map(|m| m.to_static())
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
                    //
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
                    //
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

        if self.arena[current].handle.is_none() {
            self.arena[current].handle = Some(Vec::default());
        }

        self.arena[current].segments = Some(Segments(segments));
        self.arena[current].handle.as_mut().unwrap().push(handle);

        Ok(self)
    }

    pub fn clear(&mut self) {
        self.arena = Arena::new();
        let root = self.arena.alloc(Node::default());
        self.root = root;
    }

    pub fn extend<'a, R: IntoIterator<Item = Route<'a, H>>>(&mut self, router: R) {
        for route in router {
            for handle in route.handlers {
                self.register(route.segments.clone(), handle)
                    .expect("register");
            }
        }
    }

    pub fn mount<'a, 'b, S: AsSegments<'a>, R: IntoIterator<Item = Route<'b, H>>>(
        &mut self,
        path: S,
        router: R,
    ) -> Result<(), S::Error> {
        let segments = path.as_segments()?.collect::<Vec<_>>();
        for route in router {
            let mut segments = segments.clone();
            segments.extend(route.segments);
            for handle in route.handlers {
                self.register(segments.clone(), handle).expect("register");
            }
        }

        Ok(())
    }

    pub fn find<'a: 'b, 'b, 'c, P: Params<'b>>(
        &'a self,
        path: &'b str,
        params: &'c mut P,
    ) -> Option<&'a Vec<H>> {
        let mut current_node = self.root;
        let mut catch_all: Option<&'a Named<Id<Node<H>>>> =
            self.arena[current_node].catchall.as_ref();

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
                let catch = &self.arena[catch.handle];
                return catch.handle.as_ref();
            } else {
                return None;
            }
        }

        if let Some(current) = &self.arena[current_node].handle {
            return Some(current);
        } else if let Some(catch) = catch_all {
            let star = &path[start..];
            params.set((&catch.name).into(), star.into());
            let catch = &self.arena[catch.handle];
            return catch.handle.as_ref();
        } else {
            return None;
        }
    }
}

impl<'a, H> IntoIterator for Router<H> {
    type IntoIter = IntoIter<H>;
    type Item = Route<'static, H>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.arena.into_iter())
    }
}

pub struct IntoIter<H>(id_arena::IntoIter<Node<H>, id_arena::DefaultArenaBehavior<Node<H>>>);

impl<H> Iterator for IntoIter<H> {
    type Item = Route<'static, H>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let Some((_, next)) = self.0.next() else {
                return None;
            };

            if let Some(segments) = next.segments {
                return Some(Route {
                    handlers: next.handle.unwrap(),
                    segments,
                });
            }
        }
    }
}

#[cfg(test)]
mod test {
    pub use super::*;
    use std::collections::BTreeMap;
    use std::vec;

    #[test]
    fn test() {
        let mut router = Router::new();

        router.register(&[], "root").unwrap();

        assert_eq!(
            router.find("", &mut BTreeMap::default()),
            Some(&vec!["root"])
        );
        assert_eq!(
            router.find("/", &mut BTreeMap::default()),
            Some(&vec!["root"])
        );
    }

    #[test]
    fn test_router() {
        let mut router = Router::new();

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
            router.find("path", &mut BTreeMap::default()),
            Some(&vec!["/path"])
        );
        assert_eq!(
            router.find("/path", &mut BTreeMap::default()),
            Some(&vec!["/path"])
        );
        let mut m = BTreeMap::default();
        assert_eq!(router.find("/path/10", &mut m), Some(&vec!["/path/:id"]));
        assert_eq!(m.get("id"), Some(&"10".into()));

        assert_eq!(
            router.find("/statics/filename.png", &mut BTreeMap::default()),
            Some(&vec!["/statics/*filename"])
        );
    }

    #[test]
    fn test_extend() {
        let mut router1 = Router::new();

        router1
            .register(&[Segment::constant("statics")], "statics")
            .expect("statics");

        router1
            .register(
                &[Segment::constant("statics"), Segment::constant("something")],
                "",
            )
            .expect("something");

        let mut router2 = Router::new();

        router2
            .register(&[Segment::constant("statics")], "statics2")
            .expect("statics");

        router1.extend(router2);

        assert_eq!(
            router1.find("/statics", &mut BTreeMap::default()),
            Some(&vec!["statics", "statics2"])
        );
    }

    #[test]
    fn test_mount() {
        let mut router1 = Router::new();

        router1
            .register(&[Segment::constant("statics")], "statics")
            .expect("statics");

        router1
            .register(
                &[Segment::constant("statics"), Segment::constant("something")],
                "",
            )
            .expect("something");

        let mut router2 = Router::new();

        router2
            .register(&[Segment::constant("statics")], "statics2")
            .expect("statics");

        router1.mount("/api", router2).expect("mount");

        assert_eq!(
            router1.find("/api/statics", &mut BTreeMap::default()),
            Some(&vec!["statics2"])
        );
    }
}
