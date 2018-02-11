use cssparser::{ParserInput, Parser, ParseError, BasicParseErrorKind, CowRcStr};
use cssparser::{RuleListParser, AtRuleParser, AtRuleType, QualifiedRuleParser};
use selectors::SelectorList;

mod selectors;

pub struct StyleSet {
    rules: Vec<CssRule>,
}

impl StyleSet {
    pub fn add_stylesheet(&mut self, css: &str) {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        for result in RuleListParser::new_for_stylesheet(&mut parser, VictorRulesParser) {
            // FIXME: error reporting
            if let Ok(rule) = result {
                self.rules.push(rule)
            }
        }
    }
}

enum Void {}

struct VictorRulesParser;
struct CssRule;

impl<'i> AtRuleParser<'i> for VictorRulesParser {
    type PreludeNoBlock = Void;
    type PreludeBlock = Void;
    type AtRule = CssRule;
    type Error = selectors::ParseError<'i>;

    fn parse_prelude<'t>(&mut self, name: CowRcStr<'i>, parser: &mut Parser<'i, 't>)
        -> Result<AtRuleType<Self::PreludeNoBlock, Self::PreludeBlock>, ParseError<'i, Self::Error>>
    {
        Err(parser.new_error(BasicParseErrorKind::AtRuleInvalid(name)))
    }
}

impl<'i> QualifiedRuleParser<'i> for VictorRulesParser {
    type Prelude = SelectorList<selectors::Impl>;
    type QualifiedRule = CssRule;
    type Error = selectors::ParseError<'i>;

    fn parse_prelude<'t>(&mut self, parser: &mut Parser<'i, 't>)
                         -> Result<Self::Prelude, ParseError<'i, Self::Error>>
    {
        SelectorList::parse(&selectors::Parser, parser)
    }

    fn parse_block<'t>(&mut self, _prelude: Self::Prelude, _parser: &mut Parser<'i, 't>)
                       -> Result<Self::QualifiedRule, ParseError<'i, Self::Error>>
    {
        unimplemented!()
    }
}
