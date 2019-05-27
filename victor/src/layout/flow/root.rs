use super::*;

impl crate::dom::Document {
    pub(crate) fn layout(
        &self,
        viewport: crate::primitives::Size<crate::primitives::CssPx>,
    ) -> Vec<Fragment> {
        BoxTreeRoot::construct(self).layout(viewport)
    }
}

struct BoxTreeRoot(BlockFormattingContext);

impl BoxTreeRoot {
    pub fn construct(document: &dom::Document) -> Self {
        let author_styles = &document.parse_stylesheets();
        let context = Context {
            document,
            author_styles,
        };
        let root_element = document.root_element();
        let style = style_for_element(context.author_styles, context.document, root_element, None);
        let (contains_floats, boxes) = construct_for_root_element(&context, root_element, style);
        Self(BlockFormattingContext {
            contains_floats: contains_floats == ContainsFloats::Yes,
            contents: BlockContainer::BlockLevelBoxes(boxes),
        })
    }
}

fn construct_for_root_element(
    context: &Context,
    root_element: dom::NodeId,
    style: Arc<ComputedValues>,
) -> (ContainsFloats, Vec<BlockLevelBox>) {
    let replaced = ReplacedContent::for_element(root_element, context);

    let display_inside = match style.box_.display {
        Display::None => return (ContainsFloats::No, Vec::new()),
        Display::Contents if replaced.is_some() => {
            // 'display: contents' computes to 'none' for replaced elements
            return (ContainsFloats::No, Vec::new());
        }
        // https://drafts.csswg.org/css-display-3/#transformations
        Display::Contents => DisplayInside::Flow,
        // The root element is blockified, ignore DisplayOutside
        Display::GeneratingBox(DisplayGeneratingBox::OutsideInside { inside, .. }) => inside,
    };

    if let Some(replaced) = replaced {
        let _box = match replaced {};
        #[allow(unreachable_code)]
        {
            return (ContainsFloats::No, vec![_box]);
        }
    }

    if style.box_.position.is_absolutely_positioned() {
        let contents = IndependentFormattingContext::construct(
            context,
            &style,
            display_inside,
            Contents::OfElement(root_element),
        );
        (
            ContainsFloats::No,
            vec![BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(
                AbsolutelyPositionedBox { style, contents },
            )],
        )
    } else if style.box_.float.is_floating() {
        let contents = IndependentFormattingContext::construct(
            context,
            &style,
            display_inside,
            Contents::OfElement(root_element),
        );
        (
            ContainsFloats::Yes,
            vec![BlockLevelBox::OutOfFlowFloatBox(FloatBox {
                contents,
                style,
            })],
        )
    } else {
        // FIXME: use `IndependentFormattingContext::build` and `BlockLevelBox::Independent`
        // once layout is implemented for the latter
        match display_inside {
            DisplayInside::Flow => {
                let (contents, contains_floats) = BlockContainer::construct(
                    context,
                    &style,
                    NonReplacedContents::OfElement(root_element),
                );
                (
                    contains_floats,
                    vec![BlockLevelBox::SameFormattingContextBlock { style, contents }],
                )
            }
        }
    }
}

impl BoxTreeRoot {
    fn layout(&self, viewport: crate::primitives::Size<crate::primitives::CssPx>) -> Vec<Fragment> {
        let inline_size = Length { px: viewport.width };
        // FIXME: use the document’s mode:
        // https://drafts.csswg.org/css-writing-modes/#principal-flow
        let initial_containing_block = ContainingBlock {
            inline_size,
            block_size: Some(Length {
                px: viewport.height,
            }),
            mode: (WritingMode::HorizontalTb, Direction::Ltr),
        };

        let zero = Length::zero();
        let initial_containing_block_padding = Sides {
            inline_start: zero,
            inline_end: zero,
            block_start: zero,
            block_end: zero,
        };

        let (fragments, _) = self.0.contents.layout_into_absolute_containing_block(
            &initial_containing_block,
            &initial_containing_block_padding,
        );
        fragments
    }
}
