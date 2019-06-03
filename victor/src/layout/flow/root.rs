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
            contains_floats,
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

    let contents = IndependentFormattingContext::construct(
        context,
        &style,
        display_inside,
        Contents::OfElement(root_element),
    );
    if style.box_.position.is_absolutely_positioned() {
        (
            ContainsFloats::No,
            vec![BlockLevelBox::OutOfFlowAbsolutelyPositionedBox(
                AbsolutelyPositionedBox { style, contents },
            )],
        )
    } else if let Some(anchor) = style.box_.float.anchor() {
        (
            ContainsFloats::Yes,
            vec![BlockLevelBox::OutOfFlowFloatBox(FloatBox {
                contents,
                style,
                anchor,
            })],
        )
    } else {
        (
            ContainsFloats::No,
            vec![BlockLevelBox::Independent { style, contents }],
        )
    }
}

impl BoxTreeRoot {
    fn layout(&self, viewport: crate::primitives::Size<crate::primitives::CssPx>) -> Vec<Fragment> {
        let initial_containing_block_size = Vec2 {
            inline: Length { px: viewport.width },
            block: Length {
                px: viewport.height,
            },
        };

        let initial_containing_block = ContainingBlock {
            inline_size: initial_containing_block_size.inline,
            block_size: LengthOrAuto::Length(initial_containing_block_size.block),
            // FIXME: use the document’s mode:
            // https://drafts.csswg.org/css-writing-modes/#principal-flow
            mode: (WritingMode::HorizontalTb, Direction::Ltr),
        };
        let dummy_tree_rank = 0;
        let (mut fragments, abspos, _) = self.0.layout(&initial_containing_block, dummy_tree_rank);

        let initial_containing_block = DefiniteContainingBlock {
            size: initial_containing_block_size,
            mode: initial_containing_block.mode,
        };
        fragments.par_extend(
            abspos
                .par_iter()
                .map(|a| a.layout(&initial_containing_block)),
        );

        fragments
    }
}
