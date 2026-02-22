use std::rc::Rc;

pub use taffy::{
    AlignContent,
    AlignItems,
    AlignSelf,
    AvailableSpace,
    BoxSizing,
    Clear,
    Dimension,
    Display,
    FlexDirection,
    FlexWrap,
    Float,
    GridAutoFlow,
    GridPlacement,
    GridTemplateArea,
    GridTemplateComponent,
    JustifyContent,
    JustifyItems,
    JustifySelf,
    Layout,
    LengthPercentage,
    LengthPercentageAuto,
    Line,
    Overflow,
    Position,
    Rect,
    Size,
    TextAlign,
    TrackSizingFunction,
    style_helpers::*,
};
use taffy::{
    BlockContainerStyle,
    BlockItemStyle,
    BoxGenerationMode,
    CoreStyle,
    FlexboxContainerStyle,
    FlexboxItemStyle,
    GenericGridTemplateComponent,
    GridContainerStyle,
    GridItemStyle,
    GridTemplateRepetition,
    Point,
};

use crate::{prelude::MaybeDyn, zindex::ZIndexProperties};

pub(crate) type CheapCloneStr = Rc<str>;

/// Defines if this node will receive mouse events.
#[derive(Copy, Clone, Default, Debug, PartialEq)]
pub enum PointerEvents {
    /// This node will receive pointer events.
    #[default]
    Enabled,
    /// This node will not receive any pointer events (unless a mouse capture is directly requested).
    Disabled,
}

/// A typed representation of the CSS style information for a single node.
///
/// The most important idea in flexbox is the notion of a "main" and "cross" axis, which are always perpendicular to each other.
/// The orientation of these axes are controlled via the [`FlexDirection`] field of this struct.
///
/// This struct follows the [CSS equivalent](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Flexible_Box_Layout/Basic_Concepts_of_Flexbox) directly;
/// information about the behavior on the web should transfer directly.
///
/// Detailed information about the exact behavior of each of these fields
/// can be found on [MDN](https://developer.mozilla.org/en-US/docs/Web/CSS) by searching for the field name.
/// The distinction between margin, padding and border is explained well in
/// this [introduction to the box model](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Box_Model/Introduction_to_the_CSS_box_model).
///
/// If the behavior does not match the flexbox layout algorithm on the web, please file a bug!
#[derive(Clone, PartialEq, Debug)]
pub struct Style {
    /// What layout strategy should be used?
    pub display: Display,
    /// The z-index context for this node
    pub zindex: ZIndexProperties,
    /// Whether a child is display:table or not. This affects children of block layouts.
    /// This should really be part of `Display`, but it is currently separate because table layout isn't implemented
    pub item_is_table: bool,
    /// Is it a replaced element like an image or form field?
    /// <https://drafts.csswg.org/css-sizing-3/#min-content-zero>
    pub item_is_replaced: bool,
    /// Should size styles apply to the content box or the border box of the node
    pub box_sizing: BoxSizing,

    // Overflow properties
    /// How children overflowing their container should affect layout
    pub overflow: taffy::Point<Overflow>,
    /// How much space (in points) should be reserved for the scrollbars of `Overflow::Scroll` and `Overflow::Auto` nodes.
    pub scrollbar_width: f32,

    /// Should the box be floated
    pub float: Float,
    /// Should the box clear floats
    pub clear: Clear,

    // Position properties
    /// What should the `position` value of this struct use as a base offset?
    pub position: Position,
    /// How should the position of this element be tweaked relative to the layout defined?
    pub inset: Rect<LengthPercentageAuto>,

    // Size properties
    /// Sets the initial size of the item
    pub size: Size<Dimension>,
    /// Controls the minimum size of the item
    pub min_size: Size<Dimension>,
    /// Controls the maximum size of the item
    pub max_size: Size<Dimension>,
    /// Sets the preferred aspect ratio for the item
    ///
    /// The ratio is calculated as width divided by height.
    pub aspect_ratio: Option<f32>,

    // Spacing Properties
    /// How large should the margin be on each side?
    pub margin: Rect<LengthPercentageAuto>,
    /// How large should the padding be on each side?
    pub padding: Rect<LengthPercentage>,
    /// How large should the border be on each side?
    pub border: Rect<LengthPercentage>,

    // Alignment properties
    /// How this node's children aligned in the cross/block axis?
    pub align_items: Option<AlignItems>,
    /// How this node should be aligned in the cross/block axis
    /// Falls back to the parents [`AlignItems`] if not set
    pub align_self: Option<AlignSelf>,
    /// How this node's children should be aligned in the inline axis
    pub justify_items: Option<JustifyItems>,
    /// How this node should be aligned in the inline axis
    /// Falls back to the parents [`JustifyItems`] if not set
    pub justify_self: Option<JustifySelf>,
    /// How should content contained within this item be aligned in the cross/block axis
    pub align_content: Option<AlignContent>,
    /// How should content contained within this item be aligned in the main/inline axis
    pub justify_content: Option<JustifyContent>,
    /// How large should the gaps between items in a grid or flex container be?
    pub gap: Size<LengthPercentage>,

    // Block container properties
    /// How items elements should aligned in the inline axis
    pub text_align: TextAlign,

    // Flexbox container properties
    /// Which direction does the main axis flow in?
    pub flex_direction: FlexDirection,
    /// Should elements wrap, or stay in a single line?
    pub flex_wrap: FlexWrap,

    // Flexbox item properties
    /// Sets the initial main axis size of the item
    pub flex_basis: Dimension,
    /// The relative rate at which this item grows when it is expanding to fill space
    ///
    /// 0.0 is the default value, and this value must be positive.
    pub flex_grow: f32,
    /// The relative rate at which this item shrinks when it is contracting to fit into space
    ///
    /// 1.0 is the default value, and this value must be positive.
    pub flex_shrink: f32,

    // Grid container properties
    /// Defines the track sizing functions (heights) of the grid rows
    pub grid_template_rows: Vec<GridTemplateComponent<CheapCloneStr>>,
    /// Defines the track sizing functions (widths) of the grid columns
    pub grid_template_columns: Vec<GridTemplateComponent<CheapCloneStr>>,
    /// Defines the size of implicitly created rows
    pub grid_auto_rows: Vec<TrackSizingFunction>,
    /// Defined the size of implicitly created columns
    pub grid_auto_columns: Vec<TrackSizingFunction>,
    /// Controls how items get placed into the grid for auto-placed items
    pub grid_auto_flow: GridAutoFlow,

    // Grid container named properties
    /// Defines the rectangular grid areas
    pub grid_template_areas: Vec<GridTemplateArea<CheapCloneStr>>,
    /// The named lines between the columns
    pub grid_template_column_names: Vec<Vec<CheapCloneStr>>,
    /// The named lines between the rows
    pub grid_template_row_names: Vec<Vec<CheapCloneStr>>,

    // Grid child properties
    /// Defines which row in the grid the item should start and end at
    pub grid_row: Line<GridPlacement<CheapCloneStr>>,
    /// Defines which column in the grid the item should start and end at
    pub grid_column: Line<GridPlacement<CheapCloneStr>>,

    // Pointer events
    /// Defines whether this node will receive pointer events
    pub pointer_events: PointerEvents,
}

impl Style {
    /// The [`Default`] layout, in a form that can be used in const functions
    pub const DEFAULT: Style = Style {
        display: Display::DEFAULT,
        zindex: ZIndexProperties::DEFAULT,
        item_is_table: false,
        item_is_replaced: false,
        box_sizing: BoxSizing::BorderBox,
        overflow: taffy::Point {
            x: Overflow::Visible,
            y: Overflow::Visible,
        },
        scrollbar_width: 0.0,
        float: Float::None,
        clear: Clear::None,
        position: Position::Relative,
        inset: Rect::auto(),
        margin: Rect::zero(),
        padding: Rect::zero(),
        border: Rect::zero(),
        size: Size::auto(),
        min_size: Size::auto(),
        max_size: Size::auto(),
        aspect_ratio: None,
        gap: Size::zero(),
        // Alignment
        align_items: None,
        align_self: None,
        justify_items: None,
        justify_self: None,
        align_content: None,
        justify_content: None,
        // Block
        text_align: TextAlign::Auto,
        // Flexbox
        flex_direction: FlexDirection::Row,
        flex_wrap: FlexWrap::NoWrap,
        flex_grow: 0.0,
        flex_shrink: 1.0,
        flex_basis: Dimension::AUTO,
        // Grid
        grid_template_rows: Vec::new(),
        grid_template_columns: Vec::new(),
        grid_template_areas: Vec::new(),
        grid_template_column_names: Vec::new(),
        grid_template_row_names: Vec::new(),
        grid_auto_rows: Vec::new(),
        grid_auto_columns: Vec::new(),
        grid_auto_flow: GridAutoFlow::Row,
        grid_row: Line {
            start: GridPlacement::Auto,
            end: GridPlacement::Auto,
        },
        grid_column: Line {
            start: GridPlacement::Auto,
            end: GridPlacement::Auto,
        },
        pointer_events: PointerEvents::Enabled,
    };
}

impl Default for Style {
    fn default() -> Self {
        Style::DEFAULT
    }
}

impl CoreStyle for Style {
    type CustomIdent = CheapCloneStr;

    #[inline(always)]
    fn box_generation_mode(&self) -> BoxGenerationMode {
        match self.display {
            Display::None => BoxGenerationMode::None,
            _ => BoxGenerationMode::Normal,
        }
    }
    #[inline(always)]
    fn is_block(&self) -> bool {
        matches!(self.display, Display::Block)
    }
    #[inline(always)]
    fn is_compressible_replaced(&self) -> bool {
        self.item_is_replaced
    }
    #[inline(always)]
    fn box_sizing(&self) -> BoxSizing {
        self.box_sizing
    }
    #[inline(always)]
    fn overflow(&self) -> Point<Overflow> {
        self.overflow
    }
    #[inline(always)]
    fn scrollbar_width(&self) -> f32 {
        self.scrollbar_width
    }
    #[inline(always)]
    fn position(&self) -> Position {
        self.position
    }
    #[inline(always)]
    fn inset(&self) -> Rect<LengthPercentageAuto> {
        self.inset
    }
    #[inline(always)]
    fn size(&self) -> Size<Dimension> {
        self.size
    }
    #[inline(always)]
    fn min_size(&self) -> Size<Dimension> {
        self.min_size
    }
    #[inline(always)]
    fn max_size(&self) -> Size<Dimension> {
        self.max_size
    }
    #[inline(always)]
    fn aspect_ratio(&self) -> Option<f32> {
        self.aspect_ratio
    }
    #[inline(always)]
    fn margin(&self) -> Rect<LengthPercentageAuto> {
        self.margin
    }
    #[inline(always)]
    fn padding(&self) -> Rect<LengthPercentage> {
        self.padding
    }
    #[inline(always)]
    fn border(&self) -> Rect<LengthPercentage> {
        self.border
    }
}

impl BlockContainerStyle for Style {
    #[inline(always)]
    fn text_align(&self) -> TextAlign {
        self.text_align
    }
}

impl BlockItemStyle for Style {
    #[inline(always)]
    fn is_table(&self) -> bool {
        self.item_is_table
    }

    #[inline(always)]
    fn float(&self) -> Float {
        self.float
    }

    #[inline(always)]
    fn clear(&self) -> Clear {
        self.clear
    }
}

impl FlexboxContainerStyle for Style {
    #[inline(always)]
    fn flex_direction(&self) -> FlexDirection {
        self.flex_direction
    }
    #[inline(always)]
    fn flex_wrap(&self) -> FlexWrap {
        self.flex_wrap
    }
    #[inline(always)]
    fn gap(&self) -> Size<LengthPercentage> {
        self.gap
    }
    #[inline(always)]
    fn align_content(&self) -> Option<AlignContent> {
        self.align_content
    }
    #[inline(always)]
    fn align_items(&self) -> Option<AlignItems> {
        self.align_items
    }
    #[inline(always)]
    fn justify_content(&self) -> Option<JustifyContent> {
        self.justify_content
    }
}

impl FlexboxItemStyle for Style {
    #[inline(always)]
    fn flex_basis(&self) -> Dimension {
        self.flex_basis
    }
    #[inline(always)]
    fn flex_grow(&self) -> f32 {
        self.flex_grow
    }
    #[inline(always)]
    fn flex_shrink(&self) -> f32 {
        self.flex_shrink
    }
    #[inline(always)]
    fn align_self(&self) -> Option<AlignSelf> {
        self.align_self
    }
}

impl GridContainerStyle for Style {
    type Repetition<'a>
        = &'a GridTemplateRepetition<CheapCloneStr>
    where
        Self: 'a;

    type TemplateTrackList<'a>
        = core::iter::Map<
        core::slice::Iter<'a, GridTemplateComponent<CheapCloneStr>>,
        fn(
            &'a GridTemplateComponent<CheapCloneStr>,
        ) -> GenericGridTemplateComponent<
            CheapCloneStr,
            &'a GridTemplateRepetition<CheapCloneStr>,
        >,
    >
    where
        Self: 'a;

    type AutoTrackList<'a>
        = core::iter::Copied<core::slice::Iter<'a, TrackSizingFunction>>
    where
        Self: 'a;

    type TemplateLineNames<'a>
        = core::iter::Map<
        core::slice::Iter<'a, Vec<CheapCloneStr>>,
        fn(&Vec<CheapCloneStr>) -> core::slice::Iter<'_, CheapCloneStr>,
    >
    where
        Self: 'a;
    type GridTemplateAreas<'a>
        = core::iter::Cloned<core::slice::Iter<'a, GridTemplateArea<CheapCloneStr>>>
    where
        Self: 'a;

    #[inline(always)]
    fn grid_template_rows(&self) -> Option<Self::TemplateTrackList<'_>> {
        Some(self.grid_template_rows.iter().map(|c| c.as_component_ref()))
    }
    #[inline(always)]
    fn grid_template_columns(&self) -> Option<Self::TemplateTrackList<'_>> {
        Some(
            self.grid_template_columns
                .iter()
                .map(|c| c.as_component_ref()),
        )
    }
    #[inline(always)]
    fn grid_auto_rows(&self) -> Self::AutoTrackList<'_> {
        self.grid_auto_rows.iter().copied()
    }
    #[inline(always)]
    fn grid_auto_columns(&self) -> Self::AutoTrackList<'_> {
        self.grid_auto_columns.iter().copied()
    }
    #[inline(always)]
    fn grid_auto_flow(&self) -> GridAutoFlow {
        self.grid_auto_flow
    }
    #[inline(always)]
    fn gap(&self) -> Size<LengthPercentage> {
        self.gap
    }
    #[inline(always)]
    fn align_content(&self) -> Option<AlignContent> {
        self.align_content
    }
    #[inline(always)]
    fn justify_content(&self) -> Option<JustifyContent> {
        self.justify_content
    }
    #[inline(always)]
    fn align_items(&self) -> Option<AlignItems> {
        self.align_items
    }
    #[inline(always)]
    fn justify_items(&self) -> Option<AlignItems> {
        self.justify_items
    }

    #[inline(always)]
    fn grid_template_areas(&self) -> Option<Self::GridTemplateAreas<'_>> {
        Some(self.grid_template_areas.iter().cloned())
    }

    #[inline(always)]
    fn grid_template_column_names(&self) -> Option<Self::TemplateLineNames<'_>> {
        Some(
            self.grid_template_column_names
                .iter()
                .map(|names| names.iter()),
        )
    }

    #[inline(always)]
    fn grid_template_row_names(&self) -> Option<Self::TemplateLineNames<'_>> {
        Some(
            self.grid_template_row_names
                .iter()
                .map(|names| names.iter()),
        )
    }
}

impl GridItemStyle for Style {
    #[inline(always)]
    fn grid_row(&self) -> Line<GridPlacement<CheapCloneStr>> {
        // TODO: Investigate eliminating clone
        self.grid_row.clone()
    }
    #[inline(always)]
    fn grid_column(&self) -> Line<GridPlacement<CheapCloneStr>> {
        // TODO: Investigate eliminating clone
        self.grid_column.clone()
    }
    #[inline(always)]
    fn align_self(&self) -> Option<AlignSelf> {
        self.align_self
    }
    #[inline(always)]
    fn justify_self(&self) -> Option<AlignSelf> {
        self.justify_self
    }
}

impl From<Style> for MaybeDyn<Style> {
    fn from(value: Style) -> Self {
        MaybeDyn::Static(value)
    }
}
