use crate::parser::next_segment;
use crate::parser3::into_segments;

use super::{AsSegments, Params, Segment, Segments};
#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    collections::BTreeMap as HashMap,
    string::{String, ToString},
    vec::Vec,
};
use generational_arena::{Arena, Index};
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    collections::HashMap,
    string::{String, ToString},
    vec::Vec,
};

pub trait IntoRoutes<'a, H> {
    fn into_routes(self) -> Vec<(Segments<'a>, Vec<H>)>;
}

impl<H> IntoRoutes<'static, H> for Vec<(Segments<'static>, Vec<H>)> {
    fn into_routes(self) -> Vec<(Segments<'static>, Vec<H>)> {
        self
    }
}

impl<'a, H> IntoRoutes<'a, H> for (Segments<'a>, Vec<H>) {
    fn into_routes(self) -> Vec<(Segments<'a>, Vec<H>)> {
        let mut vec = Vec::default();
        vec.push(self);
        vec
    }
}

#[derive(Debug, Clone)]
struct Named<H> {
    name: String,
    handle: H,
}

#[derive(Debug, Clone)]
struct Node<H> {
    constants: HashMap<String, Index>,
    handle: Option<Vec<H>>,
    catchall: Option<Named<Index>>,
    wildcard: Option<Named<Index>>,
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
    root: Index,
}

impl<H> Router<H> {
    pub fn new() -> Router<H> {
        let mut arena = Arena::new();
        let root = arena.insert(Node::default());
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

                    let node = self.arena.insert(Node::default());
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
                        let node = self.arena.insert(Node::default());
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
                        let node = self.arena.insert(Node::default());
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
        let root = self.arena.insert(Node::default());
        self.root = root;
    }

    pub fn extend<'a, R: IntoRoutes<'a, H>>(&mut self, router: R) {
        for route in router.into_routes() {
            for handle in route.1 {
                self.register(route.0.clone(), handle).expect("register");
            }
        }
    }

    pub fn mount<'a, 'b, S: AsSegments<'a>, R: IntoRoutes<'b, H>>(
        &mut self,
        path: S,
        router: R,
    ) -> Result<(), S::Error> {
        let segments = path.as_segments()?.collect::<Vec<_>>();
        for route in router.into_routes() {
            let mut segments = segments.clone();
            segments.extend(route.0);
            for handle in route.1 {
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
        let mut path = path;
        if path.len() > 0 && path.as_bytes()[0] == b'/' {
            path = &path[1..];
        }
        let path_len = path.char_indices().count();
        let mut current_node = self.root;
        let mut catch_all: Option<&'a Named<Index>> = None;
        let mut from = 0;
        loop {
            let start_index = from;
            let segment = next_segment(path, path_len, &mut from);
            let segment = match segment {
                Some(some) => some,
                None => {
                    //
                    if let Some(current) = &self.arena[current_node].handle {
                        return Some(current);
                    } else if let Some(catch) = catch_all {
                        params.set(Cow::Borrowed(&catch.name), (&path[start_index..]).into());
                        let catch = &self.arena[catch.handle];
                        return catch.handle.as_ref();
                    } else {
                        return None;
                    }
                }
            };

            if let Some(catch) = &self.arena[current_node].catchall {
                catch_all = Some(catch);
            }

            let sub_path = &path[segment.clone()];

            if let Some(constant) = self.arena[current_node].constants.get(sub_path) {
                current_node = *constant;
            } else if let Some(wildcard) = &self.arena[current_node].wildcard {
                params.set(wildcard.name.clone().into(), sub_path.into());
                current_node = wildcard.handle;
            } else if let Some(catch) = catch_all {
                params.set(catch.name.clone().into(), (&path[start_index..]).into());
                let catch = &self.arena[catch.handle];
                return catch.handle.as_ref();
            } else {
                return None;
            }
        }
    }

    pub fn find2<'a: 'b, 'b, 'c, P: Params<'b>>(
        &'a self,
        path: &'b str,
        params: &'c mut P,
    ) -> Option<&'a Vec<H>> {
        let mut current_node = self.root;
        let mut catch_all: Option<&'a Named<Index>> = None;

        let segments = into_segments(path);

        for seg in segments {
            if let Some(catch) = &self.arena[current_node].catchall {
                catch_all = Some(catch);
            }

            if let Some(constant) = self.arena[current_node].constants.get(seg.as_str()) {
                current_node = *constant;
            } else if let Some(wildcard) = &self.arena[current_node].wildcard {
                params.set(wildcard.name.clone().into(), seg.into_inner());
                current_node = wildcard.handle;
            } else if let Some(catch) = catch_all {
                params.set(catch.name.clone().into(), seg.into_inner());
                let catch = &self.arena[catch.handle];
                return catch.handle.as_ref();
            } else {
                return None;
            }
        }

        if let Some(current) = &self.arena[current_node].handle {
            return Some(current);
        } else if let Some(catch) = catch_all {
            // params.set(Cow::Borrowed(&catch.name), (&path[start_index..]).into());
            let catch = &self.arena[catch.handle];
            return catch.handle.as_ref();
        } else {
            return None;
        }
    }
}

impl<'a, H> IntoRoutes<'a, H> for Router<H> {
    fn into_routes(self) -> Vec<(Segments<'a>, Vec<H>)> {
        self.arena
            .into_iter()
            .filter(|m| m.segments.is_some())
            .map(|m| (m.segments.unwrap(), m.handle.unwrap()))
            .collect()
    }
}

// #[allow(unused_assignments)]
// pub(crate) fn next_segment<'a>(
//     path: &'a str,
//     path_len: usize,
//     from: &mut usize,
// ) -> Option<core::ops::Range<usize>> {
//     let mut seen = false;
//     for (i, ch) in path[*from..].char_indices() {
//         if ch != '/' {
//             continue;
//         }

//         seen = true;

//         let next = i + *from;

//         let range = Range {
//             start: *from,
//             end: next,
//         };
//         *from = next + 1;
//         return Some(range);
//     }

//     if path_len == *from {
//         return None;
//     }

//     let start = *from;
//     if !seen {
//         *from = path_len;
//     }

//     Some(Range {
//         start,
//         end: path_len,
//     })
// }

#[cfg(test)]
mod test {
    pub use super::*;

    #[cfg(not(feature = "std"))]
    use alloc::collections::BTreeMap;
    #[cfg(feature = "std")]
    use std::collections::BTreeMap;

    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(feature = "std")]
    use std::vec;

    #[test]
    fn test() {
        let mut router = Router::new();

        router.register(&[], "root").unwrap();

        assert_eq!(
            router.find2("", &mut BTreeMap::default()),
            Some(&vec!["root"])
        );
        assert_eq!(
            router.find2("/", &mut BTreeMap::default()),
            Some(&vec!["root"])
        );
    }

    #[test]
    fn test2() {
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
            router.find2("path", &mut BTreeMap::default()),
            Some(&vec!["/path"])
        );
        assert_eq!(
            router.find2("/path", &mut BTreeMap::default()),
            Some(&vec!["/path"])
        );
        let mut m = BTreeMap::default();
        assert_eq!(router.find("/path/10", &mut m), Some(&vec!["/path/:id"]));
        assert_eq!(m.get("id"), Some(&"10".into()));

        assert_eq!(
            router.find2("/statics/filename.png", &mut BTreeMap::default()),
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
