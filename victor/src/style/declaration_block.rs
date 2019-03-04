use crate::style::errors::PropertyParseErrorKind;
use crate::style::properties::{property_data_by_name, LonghandDeclaration};
use crate::style::values::{CssWideKeyword, Parse};
use cssparser::{AtRuleParser, ParseError, Parser};
use cssparser::{CowRcStr, DeclarationListParser, DeclarationParser};

pub(super) struct DeclarationBlock {
    pub declarations: Vec<LonghandDeclaration>,
}

impl DeclarationBlock {
    pub(super) fn parse(parser: &mut Parser) -> Self {
        let mut iter = DeclarationListParser::new(
            parser,
            LonghandDeclarationParser {
                block: DeclarationBlock {
                    declarations: Vec::new(),
                },
            },
        );
        while let Some(result) = iter.next() {
            let previous_len = iter.parser.block.declarations.len();
            match result {
                Ok(()) => {}
                Err(_) => {
                    iter.parser.block.declarations.truncate(previous_len);
                    // FIXME error reporting
                }
            }
        }
        iter.parser.block
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
            if let Ok(keyword) = parser.r#try(CssWideKeyword::parse) {
                for &longhand in data.longhands {
                    self.block
                        .declarations
                        .push(LonghandDeclaration::CssWide(longhand, keyword))
                }
                Ok(())
            } else {
                (data.parse)(parser, &mut self.block.declarations)
            }
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
