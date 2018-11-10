use cssparser::{Parser, ParseError, AtRuleParser, QualifiedRuleParser};
use cssparser::{CowRcStr, DeclarationListParser, DeclarationParser};
use std::rc::Rc;
use crate::style::errors::{RuleParseErrorKind, PropertyParseErrorKind};
use crate::style::properties::{PropertyDeclaration, declaration_parsing_function_by_name};
use crate::style::selectors::{self, SelectorList};

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
        let mut iter = DeclarationListParser::new(parser, PropertyDeclarationParser {
            declarations: Vec::new()
        });
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

pub struct PropertyDeclarationParser {
    declarations: Vec<PropertyDeclaration>,
}

impl<'i> DeclarationParser<'i> for PropertyDeclarationParser {
    type Declaration = ();
    type Error = PropertyParseErrorKind<'i>;

    fn parse_value<'t>(&mut self, name: CowRcStr<'i>, parser: &mut Parser<'i, 't>)
                       -> Result<Self::Declaration, ParseError<'i, Self::Error>>
    {
        if let Some(parse) = declaration_parsing_function_by_name(&name) {
            parse(parser, &mut self.declarations)
        } else {
            Err(parser.new_custom_error(PropertyParseErrorKind::UnknownProperty(name)))
        }
    }
}

impl<'i> AtRuleParser<'i> for PropertyDeclarationParser {
    type PreludeNoBlock = ();
    type PreludeBlock = ();
    type AtRule = ();
    type Error = PropertyParseErrorKind<'i>;
}
