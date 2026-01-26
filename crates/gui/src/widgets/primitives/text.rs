use std::cell::Cell;

use color::{AlphaColor, Srgb};
use euclid::default::{Point2D, Size2D};
use graphics::{
    Primitive,
    primitives::{
        self,
        TextLayoutOptions,
        text::{
            self,
            FontFamily,
            FontWeight,
            FontWidth,
            GenericFamily,
            OverflowWrap,
            WhiteSpaceCollapse,
            WordBreakStrength,
        },
    },
};
use sycamore_reactive::{MaybeDyn, ReadSignal, create_effect};
use taffy::{AvailableSpace, Layout, Size, Style};

use crate::{
    MeasureCtx,
    TreeManager,
    reexports::reactive::maybe_get_clone_untracked,
    tree::{Widget, builder::ElementBuilder},
};

pub struct Text {
    pub text: ReadSignal<String>,
    options: MaybeDyn<TextOptions>,
}
#[derive(Clone, Debug, Default)]
pub struct TextOptions {
    pub color: Option<AlphaColor<Srgb>>,
    pub font_family: Option<text::FontFamily>,
    pub font_size: Option<f32>,
    pub font_style: Option<text::FontStyle>,
    pub font_weight: Option<text::FontWeight>,
    pub font_width: Option<text::FontWidth>,
    pub line_height: Option<text::LineHeight>,
    pub overflow_wrap: Option<text::OverflowWrap>,
    pub whitespace_collapse: Option<text::WhiteSpaceCollapse>,
    pub word_break_strength: Option<text::WordBreakStrength>,
}
impl TextOptions {
    pub fn to_layout_options(self) -> TextLayoutOptions {
        TextLayoutOptions {
            font_family: self
                .font_family
                .unwrap_or(FontFamily::Generic(GenericFamily::UiSansSerif)),
            font_size: self.font_size.unwrap_or(14.),
            font_style: self
                .font_style
                .unwrap_or(primitives::text::FontStyle::Normal),
            font_weight: self.font_weight.unwrap_or(FontWeight::NORMAL),
            font_width: self.font_width.unwrap_or(FontWidth::NORMAL),
            line_height: self.line_height.unwrap_or_default(),
            overflow_wrap: self.overflow_wrap.unwrap_or(OverflowWrap::Normal),
            whitespace_collapse: self
                .whitespace_collapse
                .unwrap_or(WhiteSpaceCollapse::Collapse),
            word_break_strength: self
                .word_break_strength
                .unwrap_or(WordBreakStrength::Normal),
        }
    }
}

impl From<TextOptions> for MaybeDyn<TextOptions> {
    fn from(value: TextOptions) -> Self {
        MaybeDyn::Static(value)
    }
}

impl Widget for Text {
    fn render(&mut self, layout: &Layout, _: &Style) -> Option<Primitive> {
        let text = self.text.get_clone_untracked();
        let options = maybe_get_clone_untracked(&self.options);

        Some(Primitive::Text(primitives::Text {
            origin: Point2D::new(layout.location.x, layout.location.y),
            size: Size2D::new(layout.size.width, layout.size.height),
            text,
            color: options.color.unwrap_or(AlphaColor::WHITE),
            text_layout: options.to_layout_options(),
        }))
    }
    fn measure(
        &mut self,
        measure_ctx: &mut dyn MeasureCtx,
        known_dimensions: Size<Option<f32>>,
        available_space: Size<AvailableSpace>,
        _: &Style,
    ) -> Size<f32> {
        let text = self.text.get_clone_untracked();
        let options = maybe_get_clone_untracked(&self.options);

        let measured_size = measure_ctx.measure_text(primitives::TextMeasure {
            max_width: known_dimensions.width,
            available_space_width: match available_space.width {
                AvailableSpace::Definite(x) => primitives::AvailableSpace::Definite(x),
                _ => primitives::AvailableSpace::MaxContent,
            },
            text,
            text_layout: options.to_layout_options(),
        });
        known_dimensions.unwrap_or(measured_size)
    }
    fn debug_label(&self) -> &'static str {
        "Text"
    }

    fn focusable(&self) -> bool {
        false
    }
}

pub fn text(text: ReadSignal<String>) -> ElementBuilder<Text> {
    ElementBuilder::new_with_after_build(
        Text {
            text,
            options: MaybeDyn::Static(TextOptions::default()),
        },
        move |elem_id| {
            let mgr = TreeManager::global();

            create_effect(move || {
                text.track();
                mgr.mark_layout_dirty(elem_id);
                mgr.now();
            });
        },
    )
}

impl ElementBuilder<Text> {
    pub fn options(self, options: impl Into<MaybeDyn<TextOptions>>) -> Self {
        let options = options.into();
        let inner = Text {
            text: self.inner().text,
            options: options.clone(),
        };
        ElementBuilder::set_inner(self, inner).append_after_build(move |_| {
            let mgr = TreeManager::global();
            let options = options.clone();

            let first_run = Cell::new(true);
            create_effect(move || {
                options.track();
                if !first_run.get() {
                    log::debug!("updating div options");
                    mgr.now();
                } else {
                    first_run.set(false);
                }
            });
        })
    }
}
