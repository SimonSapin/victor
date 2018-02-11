use cssparser::{ParserInput, Parser, RuleListParser};

mod errors;
mod properties;
mod rules;
mod selectors;
mod values;

pub struct StyleSet {
    rules: Vec<rules::CssRule>,
}

impl StyleSet {
    pub fn add_stylesheet(&mut self, css: &str) {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        for result in RuleListParser::new_for_stylesheet(&mut parser, rules::RulesParser) {
            // FIXME: error reporting
            if let Ok(rule) = result {
                self.rules.push(rule)
            }
        }
    }
}
