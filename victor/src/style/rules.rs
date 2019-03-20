use crate::style::declaration_block::DeclarationBlock;
use crate::style::errors::RuleParseErrorKind;
use crate::style::selectors::{self, SelectorList};
use cssparser::{AtRuleParser, ParseError, Parser, QualifiedRuleParser, SourceLocation};
use std::sync::Arc;

pub(super) enum CssRule {
    StyleRule {
        selectors: SelectorList,

        // If this rules contains multiple (comma-separated) selectors,
        // StyleSet will want to store this declaration list as many times
        // (as positions based on the selector’s specificity)
        //
        // Use `Arc` to enable having multiple references to the `Vec` without cloning it.
        block: Arc<DeclarationBlock>,
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
        Ok(CssRule::StyleRule {
            selectors: prelude,
            block: Arc::new(DeclarationBlock::parse(parser)),
        })
    }
}

impl<'i> AtRuleParser<'i> for RulesParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = CssRule;
    type Error = RuleParseErrorKind<'i>;
}
