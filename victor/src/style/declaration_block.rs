use crate::style::errors::PropertyParseErrorKind;
use crate::style::properties::{property_data_by_name, LonghandDeclaration, PerPhase, Phase};
use crate::style::values::{CssWideKeyword, Parse};
use cssparser::{AtRuleParser, ParseError, Parser};
use cssparser::{CowRcStr, DeclarationListParser, DeclarationParser};
use std::iter::repeat;

#[derive(Default)]
pub(super) struct DeclarationBlock {
    declarations: Vec<LonghandDeclaration>,
    important: smallbitvec::SmallBitVec,
    any_important: PerPhase<bool>,
    any_normal: PerPhase<bool>,
}

impl DeclarationBlock {
    pub fn parse(parser: &mut Parser) -> Self {
        let mut iter = DeclarationListParser::new(
            parser,
            LonghandDeclarationParser {
                block: DeclarationBlock::default(),
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
        debug_assert_eq!(
            iter.parser.block.any_normal.early || iter.parser.block.any_normal.late,
            !iter.parser.block.important.all_true()
        );
        debug_assert_eq!(
            iter.parser.block.any_important.early || iter.parser.block.any_important.late,
            !iter.parser.block.important.all_false()
        );
        iter.parser.block
    }

    pub fn cascade_normal(&self, phase: &mut impl Phase) {
        self.cascade(false, self.any_normal, phase)
    }

    pub fn cascade_important(&self, phase: &mut impl Phase) {
        self.cascade(true, self.any_important, phase)
    }

    fn cascade(&self, important: bool, any: PerPhase<bool>, phase: &mut impl Phase) {
        if phase.select(any) {
            self.declarations.iter().zip(&self.important).for_each(
                move |(declaration, declaration_important)| {
                    if declaration_important == important {
                        phase.cascade(declaration)
                    }
                },
            )
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
            let mut parsed;
            if let Ok(keyword) = parser.r#try(CssWideKeyword::parse) {
                parsed = crate::style::properties::PerPhase::default();
                for &longhand in data.longhands {
                    self.block
                        .declarations
                        .push(LonghandDeclaration::CssWide(longhand, keyword));
                    if longhand.is_early() {
                        parsed.early = true
                    } else {
                        parsed.late = true
                    }
                }
            } else {
                parsed = (data.parse)(parser, &mut self.block.declarations)?
            }
            let important = cssparser::parse_important(parser).is_ok();
            let count = self.block.declarations.len() - previous_len;
            assert!(count > 0);
            self.block.important.extend(repeat(important).take(count));
            *if important {
                &mut self.block.any_important
            } else {
                &mut self.block.any_normal
            } |= parsed;
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
