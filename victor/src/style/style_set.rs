use crate::dom;
use crate::style::declaration_block::DeclarationBlock;
use crate::style::properties::{CascadeContext, ComputedValues};
use crate::style::rules::{CssRule, RulesParser};
use crate::style::selectors::{self, Selector};
use cssparser::{Parser, ParserInput, RuleListParser};
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
    fn cascade_into(
        &self,
        document: &dom::Document,
        node: dom::NodeId,
        computed: &mut ComputedValues,
        context: &CascadeContext,
    ) {
        for &(ref selector, ref block) in &self.rules {
            if selectors::matches(selector, document, node) {
                for declaration in block.declarations.iter() {
                    declaration.cascade_into(computed, context)
                }
            }
        }
    }
}

fn parse_and_apply_style_attribute(
    attr: &str,
    computed: &mut ComputedValues,
    context: &CascadeContext,
) {
    let mut input = ParserInput::new(attr);
    let mut parser = Parser::new(&mut input);
    for declaration in DeclarationBlock::parse(&mut parser).declarations {
        declaration.cascade_into(computed, context)
    }
}

pub(crate) fn cascade(
    author: &StyleSet,
    document: &dom::Document,
    node: dom::NodeId,
    parent_style: Option<&ComputedValues>,
) -> Rc<ComputedValues> {
    let element = document[node].as_element().unwrap();
    ComputedValues::new(parent_style, |computed, context| {
        USER_AGENT_STYLESHEET.with(|ua| ua.cascade_into(document, node, computed, context));
        author.cascade_into(document, node, computed, context);
        if let ns!(html) | ns!(svg) | ns!(mathml) = element.name.ns {
            if let Some(style_attr) = element.get_attr(&local_name!("style")) {
                parse_and_apply_style_attribute(style_attr, computed, context)
            }
        }
    })
}
