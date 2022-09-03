use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{alpha1, alphanumeric1},
    combinator::{map, opt, recognize},
    multi::{many0, many0_count, many1_count, separated_list1},
    sequence::pair,
    IResult,
};

#[cfg(not(feature = "std"))]
use alloc::{string::String, vec, vec::Vec};

use std::println;
#[cfg(feature = "std")]
use std::{string::String, vec, vec::Vec};

use crate::{Params, Segments};

use super::segment::Segment;

fn parse_scheme(input: &str) -> IResult<&str, &str> {
    recognize(pair(alphanumeric1, tag("://")))(input)
}

fn parse_authority(input: &str) -> IResult<&str, &str> {
    recognize(many0(alt((alpha1, tag("."), tag(":"), tag("@")))))(input)
}

fn parse_start<'a>(input: &'a str) -> IResult<&'a str, Segment<'a>> {
    let (next, ret) = recognize(pair(parse_scheme, parse_authority))(input)?;

    Ok((next, Segment::Constant(ret.into())))
}

fn parse_path_constant<'a>(input: &'a str) -> IResult<&'a str, Segment<'a>> {
    let (next, ret) = recognize(many1_count(alt((alphanumeric1, tag("_")))))(input)?;
    Ok((next, Segment::Constant(ret.into())))
}

fn parse_path_variable<'a>(input: &'a str) -> IResult<&'a str, Segment<'a>> {
    let (next, (_, identifier)) = pair(tag(":"), parse_identifier)(input)?;
    Ok((next, Segment::Parameter(identifier.into())))
}

fn parse_path_star<'a>(input: &'a str) -> IResult<&'a str, Segment<'a>> {
    let (next, (_, identifier)) = pair(tag("*"), parse_identifier)(input)?;
    Ok((next, Segment::Star(identifier.into())))
}

fn parse_identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)
}

fn parse_path_segment<'a>(input: &'a str) -> IResult<&'a str, Segment<'a>> {
    alt((parse_path_constant, parse_path_variable))(input)
}

fn parse_segments<'a>(input: &'a str) -> IResult<&'a str, Vec<Segment<'a>>> {
    let (next, segments) = alt((
        map(
            pair(
                separated_list1(tag("/"), parse_path_segment),
                opt(pair(tag("/"), parse_path_star)),
            ),
            |(mut segments, last)| {
                if let Some((_, star)) = last {
                    segments.push(star);
                }

                segments
            },
        ),
        map(parse_path_star, |ret| vec![ret]),
    ))(input)?;

    Ok((next, segments))
}

fn parse_url<'a>(input: &'a str) -> IResult<&str, Segments<'a>> {
    let (next, (start, segments)) = pair(
        alt((
            map(pair(parse_start, tag("/")), |ret| Some(ret.0)),
            map(tag("/"), |_| None),
        )),
        opt(parse_segments),
    )(input)?;

    let mut segments = segments.unwrap_or_default();

    if let Some(start) = start {
        segments.insert(0, start);
    }

    Ok((next, Segments::new(segments)))
}

#[derive(Debug)]
pub enum Error {
    Parse(nom::Err<nom::error::Error<String>>),
}

impl<'a> From<nom::Err<nom::error::Error<&'a str>>> for Error {
    fn from(err: nom::Err<nom::error::Error<&'a str>>) -> Self {
        Error::Parse(err.to_owned())
    }
}

pub fn parse<'a>(input: &'a str) -> Result<Segments<'a>, Error> {
    let (_, segments) = parse_url(input)?;
    Ok(segments)
}

pub fn next<'a>(input: &'a str) -> Result<(&'a str, &'a str), Error> {
    Ok(recognize(alt((parse_path_constant, parse_start)))(input)?)
}

pub fn into_segments<'a>(mut input: &'a str) -> impl Iterator<Item = &'a str> {
    std::iter::from_fn(move || {
        if input.len() > 1 && input.as_bytes()[0] == b'/' {
            input = &input[1..];
        }

        if input.is_empty() {
            return None;
        }

        let (next, segment) = match next(input) {
            Ok(ret) => ret,
            Err(_) => {
                return None;
            }
        };
        input = next;
        Some(segment)
    })
    .into_iter()
}

pub fn match_path<'a: 'b, 'b, 'c, P: Params<'b>>(
    segments: &Segments<'a>,
    path: &'b str,
    params: &'c mut P,
) -> bool {
    for (i, s) in into_segments(path).enumerate() {
        if i >= segments.0.len() {
            return false;
        }

        let segment = &segments.0[i];

        match segment {
            Segment::Constant(name) => {
                if *name != s {
                    return false;
                }
            }
            Segment::Parameter(n) => {
                params.set(n.clone(), s);
            }
            Segment::Star(n) => {
                params.set(n.clone(), s);
                return true;
            }
        };
    }

    true
}
#[cfg(test)]
mod test {
    use super::*;
    #[cfg(not(feature = "std"))]
    use alloc::{collections::BTreeMap, vec};
    #[cfg(feature = "std")]
    use std::{collections::BTreeMap, vec};

    #[test]
    fn test_into_segments() {
        assert_eq!(
            into_segments("/").collect::<Vec<_>>(),
            Vec::<&str>::default()
        );
        assert_eq!(
            into_segments("").collect::<Vec<_>>(),
            Vec::<&str>::default()
        );

        assert_eq!(into_segments("path").collect::<Vec<_>>(), vec!["path"]);
        assert_eq!(into_segments("path/").collect::<Vec<_>>(), vec!["path"]);
        assert_eq!(
            into_segments("path/subpath/").collect::<Vec<_>>(),
            vec!["path", "subpath"]
        );
        assert_eq!(
            into_segments("/path/subpath").collect::<Vec<_>>(),
            vec!["path", "subpath"]
        );
    }

    // #[test]
    // fn test_parse() {
    //     assert_eq!(parse("/").expect("parse"), vec![].into());
    //     assert_eq!(
    //         parse("/path").expect("parse constant"),
    //         vec![Segment::Constant("path".into())].into()
    //     );
    //     assert_eq!(
    //         parse("/path/subpath").expect("parse constant"),
    //         vec![
    //             Segment::Constant("path".into()),
    //             Segment::Constant("subpath".into())
    //         ]
    //         .into()
    //     );
    //     assert_eq!(
    //         parse("/path/:subpath").expect("parse parameter"),
    //         vec![
    //             Segment::Constant("path".into()),
    //             Segment::Parameter("subpath".into())
    //         ]
    //         .into()
    //     );
    //     assert_eq!(
    //         parse("/api/:type/:id").expect("parse parameter"),
    //         vec![
    //             Segment::Constant("api".into()),
    //             Segment::Parameter("type".into()),
    //             Segment::Parameter("id".into())
    //         ]
    //         .into()
    //     );
    //     assert_eq!(
    //         parse("/api/:type/:id/admin").expect("parse parameter"),
    //         vec![
    //             Segment::Constant("api".into()),
    //             Segment::Parameter("type".into()),
    //             Segment::Parameter("id".into()),
    //             Segment::Constant("admin".into())
    //         ]
    //         .into()
    //     );

    //     assert_eq!(
    //         parse("/*all").expect("parse star"),
    //         vec![Segment::Star("all".into())].into()
    //     );

    //     assert_eq!(
    //         parse("/path/*all").expect("parse parameter"),
    //         vec![
    //             Segment::Constant("path".into()),
    //             Segment::Star("all".into())
    //         ]
    //         .into()
    //     );

    //     assert_eq!(
    //         parse("/:path/*all").expect("parse parameter"),
    //         vec![
    //             Segment::Parameter("path".into()),
    //             Segment::Star("all".into())
    //         ]
    //         .into()
    //     );

    //     assert_eq!(
    //         parse("https://example.com/test").expect("parse parameter"),
    //         vec![
    //             Segment::Constant("https://example.com".into()),
    //             Segment::Constant("test".into())
    //         ]
    //         .into()
    //     );

    //     // assert_eq!(
    //     //     parse("*all/and-then-some"),
    //     //     Err(ParseError::CatchAllNotLast)
    //     // );
    // }

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

        assert!(match_path(
            &parse("http://test.com").expect("parse"),
            "/",
            &mut BTreeMap::default()
        ));
    }
}
