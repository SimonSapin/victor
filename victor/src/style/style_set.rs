use crate::dom;
use crate::style::declaration_block::DeclarationBlock;
use crate::style::properties::ComputedValues;
use crate::style::rules::{CssRule, RulesParser};
use crate::style::selectors::{self, Selector};
use cssparser::{Parser, ParserInput, RuleListParser};
use smallvec::SmallVec;
use std::rc::Rc;

pub struct StyleSetBuilder(StyleSet);

pub struct StyleSet {
    rules: Vec<(Selector, Rc<DeclarationBlock>)>,
}

// XXX: if we ever replace Rc with Arc for style structs,
// replace thread_local! with lazy_static! here.
thread_local! {
    static USER_AGENT_STYLESHEET: StyleSet = {
        let mut builder = StyleSetBuilder::new();
        builder.add_stylesheet(include_str!("user_agent.css"));
        builder.finish()
    };
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
                Ok(CssRule::StyleRule { selectors, block }) => {
                    for selector in selectors.0 {
                        self.0.rules.push((selector, block.clone()));
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
        self.0
            .rules
            .sort_by_key(|&(ref selector, _)| selector.specificity());
        self.0
    }
}

impl StyleSet {
    fn push_matching<'a>(
        &'a self,
        document: &dom::Document,
        node: dom::NodeId,
        mut push: impl FnMut(&'a DeclarationBlock),
    ) {
        for &(ref selector, ref block) in &self.rules {
            if selectors::matches(selector, document, node) {
                push(block)
            }
        }
    }
}

pub(crate) fn cascade(
    author: &StyleSet,
    document: &dom::Document,
    node: dom::NodeId,
    parent_style: Option<&ComputedValues>,
) -> Rc<ComputedValues> {
    let element = document[node].as_element().unwrap();
    USER_AGENT_STYLESHEET.with(|ua| {
        let style_attr_block;
        let mut matching = SmallVec::<[&DeclarationBlock; 32]>::new();
        ua.push_matching(document, node, |declaration| matching.push(declaration));
        author.push_matching(document, node, |declaration| matching.push(declaration));
        if let ns!(html) | ns!(svg) | ns!(mathml) = element.name.ns {
            if let Some(style_attr) = element.get_attr(&local_name!("style")) {
                let mut input = ParserInput::new(style_attr);
                let mut parser = Parser::new(&mut input);
                style_attr_block = DeclarationBlock::parse(&mut parser);
                matching.push(&style_attr_block);
            }
        }
        ComputedValues::new(parent_style, &matching)
    })
}
