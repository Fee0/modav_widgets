use iced::{
    advanced::{
        self,
        layout::{self, Limits, Node},
        mouse,
        renderer::Quad,
        text::{self, paragraph::Plain, LineHeight, Paragraph, Shaping, Wrapping},
        widget::tree::{self, Tag, Tree},
        Widget,
    },
    alignment::{self, Horizontal, Vertical},
    event, Background, Color, Element, Length, Padding, Pixels, Point, Rectangle, Size,
};

mod state;
use state::*;

mod utils;
pub use utils::{KeyPress, RawTable, Selection};

pub mod style;
use style::{Catalog, Style, StyleFn};

type Cell<Renderer> = Plain<<Renderer as text::Renderer>::Paragraph>;

const PAGINATION_ELLIPSIS: &str = "•••";
/// The maximum number of items on a page
const PAGE_LIMIT: usize = 25;

/// A table widget.
pub struct Table<'a, Raw, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
    Raw: RawTable,
{
    raw: &'a Raw,
    rows: usize,
    cols: usize,
    page_limit: usize,
    width: Length,
    height: Length,
    text_size: Option<Pixels>,
    font: Option<Renderer::Font>,
    header_font: Option<Renderer::Font>,
    numbering_font: Option<Renderer::Font>,
    spacing: f32,
    padding: Padding,
    cell_padding: Padding,
    status: Option<String>,
    class: Theme::Class<'a>,
    on_cell_input: Option<Box<dyn Fn(String, usize, usize) -> Message + 'a>>,
    on_cell_submit: Option<Box<dyn Fn(String, usize, usize) -> Message + 'a>>,
    on_header_input: Option<Box<dyn Fn(String, usize) -> Message + 'a>>,
    on_header_submit: Option<Box<dyn Fn(String, usize) -> Message + 'a>>,
    on_selection: Option<Box<dyn Fn(Selection) -> Message + 'a>>,
    on_keypress: Option<Box<dyn Fn(KeyPress) -> Option<Message> + 'a>>,
}

impl<'a, Raw, Message, Theme, Renderer> Table<'a, Raw, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer,
    Raw: RawTable,
{
    /// Creates a new [`Table`] widget with the given sheet.
    pub fn new(raw: &'a Raw) -> Self {
        let limit = PAGE_LIMIT.min(raw.height());
        Self {
            raw,
            rows: raw.height(),
            cols: raw.width(),
            page_limit: limit,
            width: Length::Shrink,
            height: Length::Shrink,
            text_size: None,
            padding: [10, 15].into(),
            cell_padding: [2, 5].into(),
            font: None,
            header_font: None,
            numbering_font: None,
            spacing: 10.0,
            on_cell_input: None,
            on_cell_submit: None,
            on_header_input: None,
            on_header_submit: None,
            on_selection: None,
            on_keypress: None,
            status: None,
            class: Theme::default(),
        }
    }

    /// Sets the width of the [`Table`].
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height of the [`Table`].
    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }

    // 0 causes a weird issue
    /// Sets the maximum number of rows per page for the [`Table`].
    pub fn page_limit(mut self, limit: usize) -> Self {
        self.page_limit = limit.max(1);
        self
    }

    /// Sets the text size of the [`Table`].
    pub fn text_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text_size = Some(size.into());
        self
    }

    /// Sets the [`Font`] of the [`Table`].
    pub fn font(mut self, font: Renderer::Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the [`Font`] used for headers in the [`Table`].
    pub fn header_font(mut self, font: Renderer::Font) -> Self {
        self.header_font = Some(font);
        self
    }

    /// Sets the [`Font`] used for row numbering in the [`Table`].
    pub fn numbering_font(mut self, font: Renderer::Font) -> Self {
        self.numbering_font = Some(font);
        self
    }

    /// Sets the [`Padding`] of the [`Table`].
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = padding.into();
        self
    }

    /// Sets the [`Padding`] of the cells in the [`Table`].
    pub fn cell_padding(mut self, padding: impl Into<Padding>) -> Self {
        self.cell_padding = padding.into();
        self
    }

    /// Sets the status of the [`Table`] if any.
    pub fn status_maybe(mut self, status: Option<String>) -> Self {
        self.status = status;
        self
    }

    /// Sets the message that should be produced when some text is typed a cell.
    pub fn on_cell_input(
        mut self,
        callback: impl Fn(String, usize, usize) -> Message + 'a,
    ) -> Self {
        self.on_cell_input = Some(Box::new(callback));
        self
    }

    /// Sets the message that should be produced when some text is typed a header.
    pub fn on_header_input(mut self, callback: impl Fn(String, usize) -> Message + 'a) -> Self {
        self.on_header_input = Some(Box::new(callback));
        self
    }

    /// Sets the message that should be produced when the text in a cell is submitted
    pub fn on_cell_submit(
        mut self,
        callback: impl Fn(String, usize, usize) -> Message + 'a,
    ) -> Self {
        self.on_cell_submit = Some(Box::new(callback));
        self
    }

    /// Sets the message that should be produced when the text in a header is submitted
    pub fn on_header_submit(mut self, callback: impl Fn(String, usize) -> Message + 'a) -> Self {
        self.on_header_submit = Some(Box::new(callback));
        self
    }

    /// Sets the message that should be produced when a cell selection is made.
    pub fn on_selection(mut self, callback: impl Fn(Selection) -> Message + 'a) -> Self {
        self.on_selection = Some(Box::new(callback));
        self
    }

    /// Sets the closure to produces messages on key presses.
    pub fn on_keypress(mut self, callback: impl Fn(KeyPress) -> Option<Message> + 'a) -> Self {
        self.on_keypress = Some(Box::new(callback));
        self
    }

    /// Sets the style class of the [`Table`].
    pub fn class(mut self, class: impl Into<Theme::Class<'a>>) -> Self {
        self.class = class.into();
        self
    }

    /// Sets the style of the [`Table`].
    pub fn style(mut self, style: impl Fn(&Theme) -> Style + 'a) -> Self
    where
        Theme::Class<'a>: From<StyleFn<'a, Theme>>,
    {
        self.class = (Box::new(style) as StyleFn<'a, Theme>).into();

        self
    }

    /// Ending page
    fn pages_end(&self) -> usize {
        if self.page_limit == 0 {
            return 0;
        }
        self.raw.height() / self.page_limit
    }

    fn multiple_pages(&self) -> bool {
        self.raw.height() > self.page_limit
    }
}

impl<Raw, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for Table<'_, Raw, Message, Theme, Renderer>
where
    Theme: Catalog,
    Renderer: text::Renderer + advanced::Renderer + 'static,
    Raw: RawTable,
{
    fn tag(&self) -> Tag {
        Tag::of::<State<Renderer>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer>::new())
    }

    fn size(&self) -> iced::Size<Length> {
        Size::new(self.width, self.height)
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer>>();

        state.layout(self, renderer, *limits)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &iced::advanced::renderer::Style,
        layout: iced::advanced::Layout<'_>,
        cursor: iced::advanced::mouse::Cursor,
        viewport: &iced::Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer>>();
        let bounds = layout.bounds();
        let style = theme.style(&self.class);

        let Some(clipped_viewport) = bounds.intersection(viewport) else {
            return;
        };

        if style.background.is_some() || style.border.width > 0.0 {
            <Renderer as advanced::Renderer>::fill_quad(
                renderer,
                Quad {
                    bounds,
                    border: style.border,
                    ..Default::default()
                },
                style
                    .background
                    .unwrap_or(Background::Color(Color::TRANSPARENT)),
            );
        }

        state.draw(
            self,
            renderer,
            layout,
            style,
            cursor,
            &clipped_viewport.shrink(self.padding),
        )
    }

    fn mouse_interaction(
        &self,
        state: &Tree,
        layout: layout::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        _viewport: &Rectangle,
        _renderer: &Renderer,
    ) -> advanced::mouse::Interaction {
        if !cursor.is_over(layout.bounds()) {
            return mouse::Interaction::None;
        }

        let state = state.state.downcast_ref::<State<Renderer>>();
        state.mouse_interaction(self, layout, cursor)
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: iced::Event,
        layout: layout::Layout<'_>,
        cursor: advanced::mouse::Cursor,
        renderer: &Renderer,
        _clipboard: &mut dyn advanced::Clipboard,
        shell: &mut advanced::Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = state.state.downcast_mut::<State<Renderer>>();
        state.on_update(self, renderer, event, layout, cursor, shell)
    }
}

impl<'a, Raw, Message, Theme, Renderer> From<Table<'a, Raw, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: Catalog + 'a,
    Renderer: text::Renderer + 'static,
    Raw: RawTable,
{
    fn from(value: Table<'a, Raw, Message, Theme, Renderer>) -> Self {
        Element::new(value)
    }
}

fn text<Renderer: text::Renderer>(
    content: &str,
    bounds: Size,
    font: Renderer::Font,
    horizontal: Horizontal,
    size: Pixels,
) -> text::Text<&str, Renderer::Font> {
    text::Text {
        content,
        bounds,
        size,
        line_height: LineHeight::default(),
        horizontal_alignment: horizontal,
        vertical_alignment: Vertical::Center,
        font,
        shaping: Shaping::default(),
        wrapping: Wrapping::Word,
    }
}

fn draw<Renderer>(
    renderer: &mut Renderer,
    text_color: Color,
    layout: layout::Layout<'_>,
    paragraph: &Renderer::Paragraph,
    padding: Padding,
    viewport: &Rectangle,
) where
    Renderer: text::Renderer,
{
    let bounds = layout.bounds().shrink(padding);

    let x = match paragraph.horizontal_alignment() {
        alignment::Horizontal::Left => bounds.x,
        alignment::Horizontal::Center => bounds.center_x(),
        alignment::Horizontal::Right => bounds.x + bounds.width,
    };

    let y = match paragraph.vertical_alignment() {
        alignment::Vertical::Top => bounds.y,
        alignment::Vertical::Center => bounds.center_y(),
        alignment::Vertical::Bottom => bounds.y + bounds.height,
    };

    renderer.fill_paragraph(paragraph, Point::new(x, y), text_color, *viewport);
}

fn gen_pagination(start: isize, end: isize, curr: isize) -> Vec<String> {
    let extra_left = (4 - (curr - start - 1)).max(0);
    let extra_right = (4 - (end - 1 - curr)).max(0);

    let curr_end = (curr + 3 + extra_left).min(end - 1);
    let curr_start = (curr - 3 - extra_right).max(start + 1);

    let mut output = Vec::with_capacity(11);
    output.push(start.to_string());

    if curr_start != start + 1 {
        output.push(PAGINATION_ELLIPSIS.to_owned());
    }

    for r in curr_start..=curr_end {
        output.push(r.to_string());
    }

    if curr_end != end - 1 {
        output.push(PAGINATION_ELLIPSIS.to_owned())
    }

    output.push(end.to_string());

    output
}

fn alignment_offset(
    text_bounds_width: f32,
    text_min_width: f32,
    alignment: alignment::Horizontal,
) -> f32 {
    if text_min_width > text_bounds_width {
        0.0
    } else {
        match alignment {
            alignment::Horizontal::Left => 0.0,
            alignment::Horizontal::Center => (text_bounds_width - text_min_width) / 2.0,
            alignment::Horizontal::Right => text_bounds_width - text_min_width,
        }
    }
}

fn measure_cursor_and_scroll_offset(
    paragraph: &impl text::Paragraph,
    text_bounds: Rectangle,
    cursor_index: usize,
) -> (f32, f32) {
    let grapheme_position = paragraph
        .grapheme_position(0, cursor_index)
        .unwrap_or(Point::ORIGIN);

    let offset = ((grapheme_position.x + 5.0) - text_bounds.width).max(0.0);

    (grapheme_position.x, offset)
}

fn offset<Renderer: text::Renderer>(
    text_bounds: Rectangle,
    value: &str,
    state: &State<Renderer>,
    cell: &Cell<Renderer>,
) -> f32 {
    if state.is_focused() {
        let cursor = state.cursor();

        let focus_position = match cursor.state(value) {
            utils::State::Index(i) => i,
            utils::State::Selection { end, .. } => end,
        };

        let (_, offset) = measure_cursor_and_scroll_offset(cell.raw(), text_bounds, focus_position);

        offset
    } else {
        0.0
    }
}

fn find_cursor_position<Renderer: text::Renderer>(
    text_bounds: Rectangle,
    value: &str,
    state: &State<Renderer>,
    cell: &Cell<Renderer>,
    x: f32,
) -> Option<usize> {
    let offset = offset::<Renderer>(text_bounds, value, state, cell);
    let value = value.to_string();

    let char_offset = cell
        .raw()
        .hit_test(Point::new(x + offset, text_bounds.height / 2.0))
        .map(text::Hit::cursor)?;

    let res = value[..char_offset.min(value.len())].len();

    Some(res)
}

fn word_boundary(text: &str, index: usize) -> (usize, usize) {
    if index >= text.len() {
        return (text.len(), text.len());
    }

    let chars = text.chars().collect::<Vec<char>>();
    let len = chars.len();

    if !chars[index].is_alphanumeric() && chars[index] != '_' {
        return (index, index);
    }

    let mut start = index;
    let mut end = index;

    while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
        start -= 1;
    }

    while end < len && (chars[end].is_alphanumeric() || chars[end] == '_') {
        end += 1;
    }

    if end + 1 < len {
        end += 1;
    }

    (start, end)
}
