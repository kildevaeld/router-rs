use alloc::borrow::Cow;

use alloc::vec::Vec;
use udled::{
    any,
    token::{AlphaNumeric, Opt},
    Input, Span, Tokenizer, WithSpan,
};
use udled_tokenizers::{Ident, Punctuated};

use crate::{Segment, Segments};

pub fn parse<'a>(input: &'a str) -> Result<Segments<'a>, udled::Error> {
    let mut input = Input::new(input);

    input.eat(Opt('/'))?;

    if input.eos() {
        return Ok(Segments::default());
    }

    let mut segments = if input.peek(SegmentParser)? {
        input
            .parse(Punctuated::new(SegmentParser, '/').with_trailing(true))?
            .value
    } else {
        Vec::default()
    };

    if input.eos() {
        return Ok(segments.into());
    }

    if input.peek('*')? {
        let (_, name) = input.parse(('*', Ident))?;
        segments.push(Segment::Star(name.value.into()));
    }

    Ok(segments.into())
}

struct SegmentParser;

impl Tokenizer for SegmentParser {
    type Token<'a> = Segment<'a>;

    fn to_token<'a>(
        &self,
        reader: &mut udled::Reader<'_, 'a>,
    ) -> Result<Self::Token<'a>, udled::Error> {
        if reader.peek(ContantSegmentParser)? {
            reader.parse(ContantSegmentParser)
        } else {
            reader.parse(ParamSegmentParser)
        }
    }

    fn peek(&self, reader: &mut udled::Reader<'_, '_>) -> Result<bool, udled::Error> {
        Ok(reader.peek(':')? || reader.peek(Ident)?)
    }
}

struct ContantSegmentParser;

impl Tokenizer for ContantSegmentParser {
    type Token<'a> = Segment<'a>;

    fn to_token<'a>(
        &self,
        reader: &mut udled::Reader<'_, 'a>,
    ) -> Result<Self::Token<'a>, udled::Error> {
        let parser = any!(AlphaNumeric, '_', '.', '-', '~');

        let start = reader.parse(&parser)?.span();
        loop {
            if reader.eof() {
                break;
            }

            if !reader.peek(&parser)? {
                break;
            }

            reader.eat(&parser)?;
        }

        let span = Span::new(start.start, reader.position());

        Ok(Segment::Constant(Cow::Borrowed(
            span.slice(reader.source()).unwrap(),
        )))
    }
}

struct ParamSegmentParser;

impl Tokenizer for ParamSegmentParser {
    type Token<'a> = Segment<'a>;

    fn to_token<'a>(
        &self,
        reader: &mut udled::Reader<'_, 'a>,
    ) -> Result<Self::Token<'a>, udled::Error> {
        reader.eat(":")?;

        let name = reader.parse(Ident)?;

        Ok(Segment::Parameter(Cow::Borrowed(name.as_str())))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use alloc::vec;

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
    }
}
