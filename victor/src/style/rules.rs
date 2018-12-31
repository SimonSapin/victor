use crate::style::errors::{PropertyParseErrorKind, RuleParseErrorKind};
use crate::style::properties::{property_data_by_name, LonghandDeclaration};
use crate::style::selectors::{self, SelectorList};
use crate::style::values::{CssWideKeyword, Parse};
use cssparser::{AtRuleParser, ParseError, Parser, QualifiedRuleParser, SourceLocation};
use cssparser::{CowRcStr, DeclarationListParser, DeclarationParser};
use std::rc::Rc;

pub(super) enum CssRule {
    StyleRule {
        selectors: SelectorList,

        // If this rules contains multiple (comma-separated) selectors,
        // StyleSet will want to store this declaration list as many times
        // (as positions based on the selector’s specificity)
        //
        // Use `Rc` to enable having multiple references to the `Vec` without cloning it.
        declarations: Rc<Vec<LonghandDeclaration>>,
    },
}

pub(super) struct RulesParser;

impl<'i> QualifiedRuleParser<'i> for RulesParser {
    type Prelude = SelectorList;
    type QualifiedRule = CssRule;
    type Error = RuleParseErrorKind<'i>;

    fn parse_prelude<'t>(
        &mut self,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i, Self::Error>> {
        SelectorList::parse(&selectors::Parser, parser)
    }

    fn parse_block<'t>(
        &mut self,
        prelude: Self::Prelude,
        _location: SourceLocation,
        parser: &mut Parser<'i, 't>,
    ) -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>> {
        let mut iter = DeclarationListParser::new(
            parser,
            LonghandDeclarationParser {
                declarations: Vec::new(),
            },
        );
        while let Some(result) = iter.next() {
            let previous_len = iter.parser.declarations.len();
            match result {
                Ok(()) => {}
                Err(_) => {
                    iter.parser.declarations.truncate(previous_len);
                    // FIXME error reporting
                }
            }
        }

        Ok(CssRule::StyleRule {
            selectors: prelude,
            declarations: Rc::new(iter.parser.declarations),
        })
    }
}

impl<'i> AtRuleParser<'i> for RulesParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = CssRule;
    type Error = RuleParseErrorKind<'i>;
}

pub(super) struct LonghandDeclarationParser {
    declarations: Vec<LonghandDeclaration>,
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
                    self.declarations
                        .push(LonghandDeclaration::CssWide(longhand, keyword))
                }
                Ok(())
            } else {
                (data.parse)(parser, &mut self.declarations)
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
