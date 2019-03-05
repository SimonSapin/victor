use crate::style::errors::PropertyParseErrorKind;
use crate::style::properties::{property_data_by_name, LonghandDeclaration};
use crate::style::values::{CssWideKeyword, Parse};
use cssparser::{AtRuleParser, ParseError, Parser};
use cssparser::{CowRcStr, DeclarationListParser, DeclarationParser};
use std::iter::repeat;

pub(super) struct DeclarationBlock {
    declarations: Vec<LonghandDeclaration>,
    important: smallbitvec::SmallBitVec,
    any_normal: bool,
    any_important: bool,
}

impl DeclarationBlock {
    pub fn parse(parser: &mut Parser) -> Self {
        let mut iter = DeclarationListParser::new(
            parser,
            LonghandDeclarationParser {
                block: DeclarationBlock {
                    declarations: Vec::new(),
                    important: smallbitvec::SmallBitVec::new(),
                    any_normal: false,
                    any_important: false,
                },
            },
        );
        loop {
            let previous_len = iter.parser.block.declarations.len();
            let result = if let Some(r) = iter.next() { r } else { break };
            match result {
                Ok(()) => {}
                Err(_) => {
                    assert!(iter.parser.block.declarations.len() == previous_len);
                    // FIXME error reporting
                }
            }
            debug_assert_eq!(
                iter.parser.block.declarations.len(),
                iter.parser.block.important.len()
            );
        }
        {
            let block = &iter.parser.block;
            debug_assert_eq!(block.any_normal, !block.important.all_true());
            debug_assert_eq!(block.any_important, !block.important.all_false());
        }
        iter.parser.block
    }

    pub fn for_each_normal(&self, f: &mut impl FnMut(&LonghandDeclaration)) {
        self.for_each(false, self.any_normal, f)
    }

    pub fn for_each_important(&self, f: &mut impl FnMut(&LonghandDeclaration)) {
        self.for_each(true, self.any_important, f)
    }

    fn for_each(&self, important: bool, any: bool, f: &mut impl FnMut(&LonghandDeclaration)) {
        if any {
            self.declarations
                .iter()
                .zip(&self.important)
                .for_each(move |(d, i)| {
                    if i == important {
                        f(d)
                    }
                })
        }
    }
}

struct LonghandDeclarationParser {
    block: DeclarationBlock,
}

impl<'i> DeclarationParser<'i> for LonghandDeclarationParser {
    type Declaration = ();
    type Error = PropertyParseErrorKind<'i>;

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, ParseError<'i, Self::Error>> {
        if let Some(data) = property_data_by_name(&name) {
            let previous_len = self.block.declarations.len();
            if let Ok(keyword) = parser.r#try(CssWideKeyword::parse) {
                for &longhand in data.longhands {
                    self.block
                        .declarations
                        .push(LonghandDeclaration::CssWide(longhand, keyword))
                }
            } else {
                (data.parse)(parser, &mut self.block.declarations)?
            }
            let important = cssparser::parse_important(parser).is_ok();
            let count = self.block.declarations.len() - previous_len;
            assert!(count > 0);
            self.block.any_normal |= !important;
            self.block.any_important |= important;
            self.block.important.extend(repeat(important).take(count));
            Ok(())
        } else {
            Err(parser.new_custom_error(PropertyParseErrorKind::UnknownProperty(name)))
        }
    }
}

impl<'i> AtRuleParser<'i> for LonghandDeclarationParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = ();
    type Error = PropertyParseErrorKind<'i>;
}
