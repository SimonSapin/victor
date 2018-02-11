use cssparser::{Parser, ParseError, AtRuleParser, QualifiedRuleParser, DeclarationListParser};
use std::rc::Rc;
use style::errors::RuleParseErrorKind;
use style::properties::{PropertyDeclaration, PropertyDeclarationParser};
use style::selectors::{self, SelectorList};

pub enum CssRule {
    StyleRule {
        selectors: SelectorList,

        // If this rules contains multiple (comma-separated) selectors,
        // StyleSet will want to store this declaration list as many times
        // (as positions based on the selector’s specificity)
        //
        // Use `Rc` to enable having multiple references to the `Vec` without cloning it.
        declarations: Rc<Vec<PropertyDeclaration>>,
    }
}

pub struct RulesParser;

impl<'i> QualifiedRuleParser<'i> for RulesParser {
    type Prelude = SelectorList;
    type QualifiedRule = CssRule;
    type Error = RuleParseErrorKind<'i>;

    fn parse_prelude<'t>(&mut self, parser: &mut Parser<'i, 't>)
                         -> Result<Self::Prelude, ParseError<'i, Self::Error>>
    {
        SelectorList::parse(&selectors::Parser, parser)
    }

    fn parse_block<'t>(&mut self, prelude: Self::Prelude, parser: &mut Parser<'i, 't>)
                       -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>>
    {
        let decls = DeclarationListParser::new(parser, PropertyDeclarationParser);

        // FIXME error reporting
        let decls = decls.filter_map(Result::ok);

        Ok(CssRule::StyleRule {
            selectors: prelude,
            declarations: Rc::new(decls.collect()),
        })
    }
}

impl<'i> AtRuleParser<'i> for RulesParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = CssRule;
    type Error = RuleParseErrorKind<'i>;
}
