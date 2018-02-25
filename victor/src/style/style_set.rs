use cssparser::{ParserInput, Parser, RuleListParser};
use dom::NodeRef;
use std::rc::Rc;
use style::properties::{PropertyDeclaration, ComputedValues};
use style::rules::{CssRule, RulesParser};
use style::selectors::{self, Selector};

pub struct StyleSetBuilder(StyleSet);

pub struct StyleSet {
    rules: Vec<(Selector, Rc<Vec<PropertyDeclaration>>)>,
}

impl StyleSetBuilder {
    pub fn new() -> Self {
        StyleSetBuilder(StyleSet { rules: Vec::new() })
    }

    pub fn add_stylesheet(&mut self, css: &str) {
        let mut input = ParserInput::new(css);
        let mut parser = Parser::new(&mut input);
        for result in RuleListParser::new_for_stylesheet(&mut parser, RulesParser) {
            match result {
                Ok(CssRule::StyleRule { selectors, declarations }) => {
                    for selector in selectors.0 {
                        self.0.rules.push((selector, declarations.clone()));
                    }
                }
                Err(_) => {
                    // FIXME: error reporting
                }
            }
        }
    }

    pub fn finish(mut self) -> StyleSet {
        // Sort stability preserves document order for rules of equal specificity
        self.0.rules.sort_by_key(|&(ref selector, _)| selector.specificity());
        self.0
    }
}

impl StyleSet {
    pub fn cascade(&self, node: NodeRef, parent_style: Option<&ComputedValues>) -> ComputedValues {
        let mut computed = ComputedValues::new(parent_style);
        for &(ref selector, ref declarations) in &self.rules {
            if selectors::matches(selector, node) {
                for declaration in declarations.iter() {
                    declaration.cascade_into(&mut computed)
                }
            }
        }
        computed
    }
}
