use core::ops::Range;

use crate::{Params, Segment};

pub(crate) fn into_segments<'a>(input: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
    let mut progress = 0usize;
    let len = input.len();

    if input.starts_with("/") {
        progress += 1;
    }

    core::iter::from_fn(move || {
        if progress == input.len() {
            return None;
        }

        let mut current = progress;

        let mut chars = input[progress..].chars().peekable();

        loop {
            let next = match chars.next() {
                Some(ret) => ret,
                None => break,
            };

            current += 1;

            if next == '/' {
                if let Some(_) = chars.next_if(|ch| ch == &'/') {
                    current += 1;
                } else {
                    current -= 1;
                    break;
                }
            }
        }

        if progress == current {
            None
        } else {
            let rg = progress..current;
            progress = current;
            if progress != len {
                progress += 1;
            }
            Some(rg)
        }
    })
}

pub fn match_path<'a, 'c, S: AsRef<[Segment<'a>]>, P: Params>(
    segments: S,
    mut path: &str,
    params: &'c mut P,
) -> bool {
    if path.len() != 0 && path.as_bytes()[0] == b'/' {
        path = &path[1..];
    }

    let segments = segments.as_ref();

    if path.len() == 0 && segments.len() == 0 {
        return true;
    } else if path.len() == 0 {
        return false;
    }

    let mut segments = segments.iter();
    let mut current: Option<&Segment<'_>> = None;

    let mut iter = into_segments(path);

    loop {
        let range = match iter.next() {
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
                params.set(n.clone(), (&path[range]).into());
            }
            Some(Segment::Star(n)) => {
                params.set(n.clone(), (&path[range]).into());
                return true;
            }
            None => return false,
        };
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use alloc::{collections::BTreeMap, string::String, string::ToString, vec, vec::Vec};

    use crate::parser::parse;

    macro_rules! segments {
        ($url: literal => $($segs: literal),*) => {
            assert_eq!(
                into_segments($url).map(|m| $url[m].to_string()).collect::<Vec<_>>(),
                vec![$($segs.to_string()),*]
            );

        };
        ($url: literal) => {
            assert_eq!(
                into_segments($url).map(|m| $url[m].to_string()).collect::<Vec<_>>(),
                Vec::<String>::default()
            );

        };
    }

    #[test]
    fn test_into_segments() {
        segments!("/");
        segments!("");
        segments!("/path" => "path");
        segments!("path" => "path");
        segments!("path/" => "path");
        segments!("/path/" => "path");
        segments!("/path/subpath" => "path", "subpath");
        segments!("/path/subpath/" => "path", "subpath");
        segments!("path/subpath/" => "path", "subpath");
        segments!("https://test.com/test/path/subpath/" => "https://test.com", "test", "path", "subpath");
    }

    #[test]
    fn test_match_path() {
        assert!(match_path(
            parse("/").expect("parse"),
            "",
            &mut BTreeMap::default()
        ));
        assert!(match_path(
            parse("/").expect("parse"),
            "/",
            &mut BTreeMap::default()
        ));
        assert!(!match_path(
            parse("/").expect("parse"),
            "/withpath",
            &mut BTreeMap::default()
        ));
        assert!(match_path(
            parse("/subpath").expect("parse"),
            "/subpath",
            &mut BTreeMap::default()
        ));
        let mut params = BTreeMap::default();
        assert!(match_path(
            parse("/:subpath").expect("parse"),
            "/ost",
            &mut params
        ));
        assert_eq!(params.get("subpath").map(|m| m), Some(&"ost".into()));
        assert!(!match_path(
            parse("/:subpath").expect("parse"),
            "/ost/boef",
            &mut BTreeMap::default()
        ));
    }
}
