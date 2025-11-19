use alloc::vec::Vec;
use udled::{
    any,
    tokenizers::{AlphaNumeric, Opt, Puntuated},
    AsChar, AsSlice, AsStr, Buffer, Input, Tokenizer, TokenizerExt, EOF,
};
use udled_tokenizers::Ident;

use crate::{Segment, Segments};

pub fn parse<'a>(input: &'a str) -> Result<Segments<'a>, udled::Error> {
    let mut input = Input::new(input);

    input.eat(Opt::new('/'))?;

    if input.is(EOF) {
        return Ok(Segments::default());
    }

    let mut segments = input
        .parse(Puntuated::new(SegmentParser, '/').optional())?
        .map(|m| m.into_items().collect::<Vec<_>>())
        .unwrap_or_default();

    input.eat('/'.optional())?;

    if input.is('*') {
        let (_, name) = input.parse(('*', Ident))?;
        segments.push(Segment::Star(name.value.into()));
    }

    Ok(segments.into())
}

struct SegmentParser;

impl<'input, B> Tokenizer<'input, B> for SegmentParser
where
    B: Buffer<'input>,
    B::Item: AsChar,
    B::Source: AsSlice<'input>,
    <B::Source as AsSlice<'input>>::Slice: AsStr<'input>,
{
    type Token = Segment<'input>;

    fn to_token<'a>(
        &self,
        reader: &mut udled::Reader<'_, 'input, B>,
    ) -> Result<Self::Token, udled::Error> {
        if reader.is(':') {
            let (_, ident) = reader.parse((':', Ident))?;
            Ok(Segment::Parameter(ident.value.as_str().into()))
        } else {
            let path = reader.parse(any!(AlphaNumeric, '_', '.', '-', '~').many().slice())?;
            Ok(Segment::Constant(path.value.as_str().into()))
        }
    }

    fn peek(&self, reader: &mut udled::Reader<'_, 'input, B>) -> bool {
        reader.is(':') || reader.is(Ident)
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
