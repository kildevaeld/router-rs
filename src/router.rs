use super::{AsSegments, Params, Segment};
#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    collections::BTreeMap as HashMap,
    string::{String, ToString},
    vec::Vec,
};
use core::ops::Range;
use id_arena::{Arena, Id};
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    collections::HashMap,
    string::{String, ToString},
    vec::Vec,
};

type NodeId<H> = Id<Node<H>>;

#[derive(Debug, Clone)]
struct Named<H> {
    name: String,
    handle: H,
}

#[derive(Debug, Clone)]
struct Node<H> {
    constants: HashMap<String, NodeId<H>>,
    handle: Option<Vec<H>>,
    catchall: Option<Named<NodeId<H>>>,
    wildcard: Option<Named<NodeId<H>>>,
}

impl<H> Default for Node<H> {
    fn default() -> Node<H> {
        Node {
            constants: HashMap::default(),
            handle: None,
            catchall: None,
            wildcard: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Router<H> {
    arena: Arena<Node<H>>,
    root: NodeId<H>,
}

impl<H> Router<H> {
    pub fn new() -> Router<H> {
        let mut arena = Arena::new();
        let root = arena.alloc(Node::default());
        Router { arena, root }
    }

    pub fn register<'a, S: AsSegments<'a> + 'a>(
        &mut self,
        path: S,
        handle: H,
    ) -> Result<&mut Self, S::Error> {
        let mut current = self.root;

        let segments = path.as_segments()?;

        'path: for segment in segments {
            //
            match segment {
                Segment::Constant(path) => {
                    //
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
                        // TODO Tjek is names is the same
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

        self.arena[current].handle.as_mut().unwrap().push(handle);

        Ok(self)
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
        let mut catch_all: Option<&'a Named<NodeId<H>>> = None;
        let mut from = 0;
        // let mut params = HashMap::new();
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
                        params.set(Cow::Borrowed(&catch.name), &path[start_index..]);
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
                params.set(wildcard.name.clone().into(), sub_path);
                current_node = wildcard.handle;
            } else if let Some(catch) = catch_all {
                params.set(catch.name.clone().into(), &path[start_index..]);
                let catch = &self.arena[catch.handle];
                return catch.handle.as_ref();
            } else {
                return None;
            }
        }
    }
}

#[allow(unused_assignments)]
pub(crate) fn next_segment<'a>(
    path: &'a str,
    path_len: usize,
    from: &mut usize,
) -> Option<core::ops::Range<usize>> {
    let mut seen = false;
    for (i, ch) in path[*from..].char_indices() {
        if ch != '/' {
            continue;
        }

        seen = true;

        let next = i + *from;

        let range = Range {
            start: *from,
            end: next,
        };
        *from = next + 1;
        return Some(range);
    }

    if path_len == *from {
        return None;
    }

    let start = *from;
    if !seen {
        *from = path_len;
    }

    Some(Range {
        start,
        end: path_len,
    })
}

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
            router.find("", &mut BTreeMap::default()),
            Some(&vec!["root"])
        );
        assert_eq!(
            router.find("/", &mut BTreeMap::default()),
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
            router.find("path", &mut BTreeMap::default()),
            Some(&vec!["/path"])
        );
        assert_eq!(
            router.find("/path", &mut BTreeMap::default()),
            Some(&vec!["/path"])
        );
        let mut m = BTreeMap::default();
        assert_eq!(router.find("/path/10", &mut m), Some(&vec!["/path/:id"]));
        assert_eq!(m.get("id"), Some(&"10"));

        assert_eq!(
            router.find("/statics/filename.png", &mut BTreeMap::default()),
            Some(&vec!["/statics/*filename"])
        );
    }
}
