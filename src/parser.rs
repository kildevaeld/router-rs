use super::{router::next_segment, Segment};
#[cfg(not(feature = "std"))]
use alloc::{borrow::Cow, collections::BTreeMap, vec::Vec};
use core::ops::Range;
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    vec::Vec,
};

pub trait Params<'a> {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str);
}

impl<'a> Params<'a> for BTreeMap<Cow<'a, str>, &'a str> {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str) {
        self.insert(key, value);
    }
}

#[cfg(feature = "std")]
impl<'a> Params<'a> for HashMap<Cow<'a, str>, &'a str> {
    fn set(&mut self, key: Cow<'a, str>, value: &'a str) {
        self.insert(key, value);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseError {
    MissingVarName { pos: Range<usize> },
    CatchAllNotLast,
}

pub fn parse<'a>(mut path: &'a str) -> Result<Vec<Segment<'a>>, ParseError> {
    if !path.is_empty() && path.as_bytes()[0] == b'/' {
        path = &path[1..];
    }

    let path_len = path.char_indices().count();
    if path_len == 0 {
        return Ok(Vec::default());
    }

    let mut segments = Vec::default();
    let mut idx = 0;
    loop {
        let segment = match next_segment(path, path_len, &mut idx) {
            Some(segment) => segment,
            None => break,
        };

        if segment.len() == 0 {
            continue;
        }

        let subpath = &path[segment.clone()];

        if subpath.starts_with(":") || subpath.starts_with("*") {
            let name = &subpath[1..];
            if name.len() == 0 {
                return Err(ParseError::MissingVarName { pos: segment });
            }

            if subpath.starts_with("*") {
                if idx != path_len {
                    return Err(ParseError::CatchAllNotLast);
                }
                segments.push(Segment::Star(name.into()));
            } else {
                segments.push(Segment::Parameter(name.into()));
            }
        } else {
            segments.push(Segment::Constant(subpath.into()));
        }
    }

    Ok(segments)
}

pub fn match_path<'a: 'b, 'b, 'c, P: Params<'b>>(
    segments: &[Segment<'a>],
    mut path: &'b str,
    params: &'c mut P,
) -> bool {
    if path.len() != 0 && path.as_bytes()[0] == b'/' {
        path = &path[1..];
    }

    if path.len() == 0 && segments.len() == 0 {
        return true;
    } else if path.len() == 0 {
        return false;
    }
    let path_len = path.char_indices().count();
    let mut idx = 0;
    let mut segments = segments.iter();
    let mut current: Option<&Segment<'_>> = None;
    loop {
        let range = match next_segment(path, path_len, &mut idx) {
            None => return current.is_some() && segments.next().is_none(),
            Some(range) => range,
        };

        current = segments.next();

        match current {
            Some(Segment::Constant(name)) => {
                if *name != &path[range] {
                    return false;
                }
            }
            Some(Segment::Parameter(n)) => {
                params.set(n.clone(), &path[range]);
            }
            Some(Segment::Star(n)) => {
                params.set(n.clone(), &path[range]);
                return true;
            }
            None => return false,
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::vec;
    #[cfg(feature = "std")]
    use std::vec;

    #[test]
    fn test_parse() {
        assert_eq!(parse("/").expect("parse"), vec![]);
        assert_eq!(
            parse("/path").expect("parse constant"),
            vec![Segment::Constant("path".into())]
        );
        assert_eq!(
            parse("/path/subpath").expect("parse constant"),
            vec![
                Segment::Constant("path".into()),
                Segment::Constant("subpath".into())
            ]
        );
        assert_eq!(
            parse("/path/:subpath").expect("parse parameter"),
            vec![
                Segment::Constant("path".into()),
                Segment::Parameter("subpath".into())
            ]
        );
        assert_eq!(
            parse("/api/:type/:id").expect("parse parameter"),
            vec![
                Segment::Constant("api".into()),
                Segment::Parameter("type".into()),
                Segment::Parameter("id".into())
            ]
        );
        assert_eq!(
            parse("/api/:type/:id/admin").expect("parse parameter"),
            vec![
                Segment::Constant("api".into()),
                Segment::Parameter("type".into()),
                Segment::Parameter("id".into()),
                Segment::Constant("admin".into())
            ]
        );

        assert_eq!(
            parse("*all").expect("parse parameter"),
            vec![Segment::Star("all".into())]
        );

        assert_eq!(
            parse("path/*all").expect("parse parameter"),
            vec![
                Segment::Constant("path".into()),
                Segment::Star("all".into())
            ]
        );

        assert_eq!(
            parse(":path/*all").expect("parse parameter"),
            vec![
                Segment::Parameter("path".into()),
                Segment::Star("all".into())
            ]
        );

        assert_eq!(
            parse("*all/and-then-some"),
            Err(ParseError::CatchAllNotLast)
        );
    }

    #[test]
    fn test_match_path() {
        assert!(match_path(
            &parse("/").expect("parse"),
            "",
            &mut BTreeMap::default()
        ));
        assert!(match_path(
            &parse("/").expect("parse"),
            "/",
            &mut BTreeMap::default()
        ));
        assert!(!match_path(
            &parse("/").expect("parse"),
            "/withpath",
            &mut BTreeMap::default()
        ));
        assert!(match_path(
            &parse("/subpath").expect("parse"),
            "/subpath",
            &mut BTreeMap::default()
        ));
        let mut params = BTreeMap::default();
        assert!(match_path(
            &parse("/:subpath").expect("parse"),
            "/ost",
            &mut params
        ));
        assert_eq!(params.get("subpath").map(|m| *m), Some("ost"));
        assert!(!match_path(
            &parse("/:subpath").expect("parse"),
            "/ost/boef",
            &mut BTreeMap::default()
        ));
    }
}
