use super::Segment;
use crate::{Params, Segments};
use alloc::{vec, vec::Vec};
use core::ops::Range;

peg::parser! {
    grammar parser() for str {

        pub rule parse() -> Segments<'input>
            = segments:(s:start() i:("/" i:parse_path() { i })? {
                let mut i = i.unwrap_or_default();
                i.insert(0, s);
                i
            }
            / "/"? i:parse_path() { i }) star:("/" s:parse_star_segment() { s })? "/"? {
                let mut segments = segments;
                if let Some(i) = star {
                    segments.push(i);
                }

                segments.into()
            }
            / "/"? s:parse_star_segment()?  "/"? { Segments(s.map(|m| vec![m]).unwrap_or_default()) }



        rule start() -> Segment<'input>
            = i:$(identifier() ":" authority()?) {
                Segment::Constant(i.into())
            }

        rule authority()
            = "//" (identifier() "@" )?  ['a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '.']+


        rule parse_path() -> Vec<Segment<'input>>
            = p:parse_path_segment() ++ "/" {
                p
            }

        rule parse_path_segment() ->    Segment<'input>
            = p:( parse_constant_segment() / parse_variable_segment() )  { p }

        rule parse_constant_segment() -> Segment<'input>
            = i:$identifier() {
                Segment::Constant(i.into())
            }

        rule parse_variable_segment() -> Segment<'input>
            = ":" i:$identifier() {
                Segment::Parameter(i.into())
            }


        rule parse_star_segment() -> Segment<'input>
            = "*" i:$identifier() {
                Segment::Star(i.into())
            }


        rule identifier()
            = (['a'..='z' | 'A'..='Z' | '_' | '0'..='9'])+
    }
}

pub type ParseError = peg::error::ParseError<peg::str::LineCol>;

pub use parser::parse;

pub fn into_segments<'a>(input: &'a str) -> impl Iterator<Item = Range<usize>> + 'a {
    let mut progress = 0usize;
    let len = input.len();

    if input.starts_with("/") {
        progress += 1;
    }

    std::iter::from_fn(move || {
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

pub fn match_path<'a: 'b, 'b, 'c, S: AsRef<[Segment<'a>]>, P: Params<'b>>(
    segments: S,
    mut path: &'b str,
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
    #[cfg(not(feature = "std"))]
    use alloc::{collections::BTreeMap, string::ToString, vec, String};
    #[cfg(feature = "std")]
    use std::{collections::BTreeMap, string::String, string::ToString, vec};

    use parser::parse;

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
    fn test_parse() {
        assert_eq!(parse("/").expect("parse"), vec![].into());
        assert_eq!(parse("").expect("parse"), vec![].into());
        assert_eq!(
            parse("*all").expect("parse"),
            vec![Segment::Star("all".into())].into()
        );

        assert_eq!(
            parse("/path").expect("parse constant"),
            vec![Segment::Constant("path".into())].into()
        );
        assert_eq!(
            parse("/path/subpath").expect("parse constant"),
            vec![
                Segment::Constant("path".into()),
                Segment::Constant("subpath".into())
            ]
            .into()
        );
        assert_eq!(
            parse("/path/:subpath").expect("parse parameter"),
            vec![
                Segment::Constant("path".into()),
                Segment::Parameter("subpath".into())
            ]
            .into()
        );
        assert_eq!(
            parse("/api/:type/:id").expect("parse parameter"),
            vec![
                Segment::Constant("api".into()),
                Segment::Parameter("type".into()),
                Segment::Parameter("id".into())
            ]
            .into()
        );
        assert_eq!(
            parse("/api/:type/:id/admin").expect("parse parameter"),
            vec![
                Segment::Constant("api".into()),
                Segment::Parameter("type".into()),
                Segment::Parameter("id".into()),
                Segment::Constant("admin".into())
            ]
            .into()
        );

        assert_eq!(
            parse("/*all").expect("parse star"),
            vec![Segment::Star("all".into())].into()
        );

        assert_eq!(
            parse("/path/*all").expect("parse parameter"),
            vec![
                Segment::Constant("path".into()),
                Segment::Star("all".into())
            ]
            .into()
        );

        assert_eq!(
            parse("/:path/*all").expect("parse parameter"),
            vec![
                Segment::Parameter("path".into()),
                Segment::Star("all".into())
            ]
            .into()
        );

        assert_eq!(
            parse("https://example.com/").expect("parse parameter"),
            vec![Segment::Constant("https://example.com".into()),].into()
        );

        assert_eq!(
            parse("https://example.com/test").expect("parse parameter"),
            vec![
                Segment::Constant("https://example.com".into()),
                Segment::Constant("test".into())
            ]
            .into()
        );

        // assert_eq!(
        //     parse("*all/and-then-some"),
        //     Err(ParseError::CatchAllNotLast)
        // );
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
        assert_eq!(params.get("subpath").map(|m| m), Some(&"ost".into()));
        assert!(!match_path(
            &parse("/:subpath").expect("parse"),
            "/ost/boef",
            &mut BTreeMap::default()
        ));

        assert!(match_path(
            &parse("http://test.com/").expect("parse"),
            "http://test.com/",
            &mut BTreeMap::default()
        ));

        assert!(match_path(
            &parse("http://test.com/:name").expect("parse"),
            "http://test.com/hello_world",
            &mut BTreeMap::default()
        ));
    }
}
