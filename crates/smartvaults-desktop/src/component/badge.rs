// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::advanced::layout::{Limits, Node};
use iced::advanced::mouse::Cursor;
use iced::advanced::renderer;
use iced::advanced::widget::tree::Tree;
use iced::advanced::{Clipboard, Layout, Shell, Widget};
use iced::{event, Alignment, Background, Color, Element, Event, Length, Point, Rectangle, Theme};

use crate::theme::color::{BLACK, GREEN, LIGHT_BLUE, ORANGE2, RED, WHITE};

/// The ratio of the border radius.
const BORDER_RADIUS_RATIO: f32 = 34.0 / 15.0;

pub struct Badge<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// The padding of the [`Badge`].
    padding: u16,
    /// The width of the [`Badge`].
    width: Length,
    /// The height of the [`Badge`].
    height: Length,
    /// The horizontal alignment of the [`Badge`].
    horizontal_alignment: Alignment,
    /// The vertical alignment of the [`Badge`].
    vertical_alignment: Alignment,
    /// The style of the [`Badge`].
    style: <Renderer::Theme as StyleSheet>::Style,
    /// The content [`Element`] of the [`Badge`].
    content: Element<'a, Message, Renderer>,
}

impl<'a, Message, Renderer> Badge<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
    Renderer::Theme: StyleSheet,
{
    /// Creates a new [`Badge`] with the given content.
    ///
    /// It expects:
    ///     * the content [`Element`] to display in the [`Badge`].
    pub fn new<T>(content: T) -> Self
    where
        T: Into<Element<'a, Message, Renderer>>,
    {
        Badge {
            padding: 5,
            width: Length::Shrink,
            height: Length::Shrink,
            horizontal_alignment: Alignment::Center,
            vertical_alignment: Alignment::Center,
            style: <Renderer::Theme as StyleSheet>::Style::default(),
            content: content.into(),
        }
    }

    /// Sets the horizontal alignment of the content of the [`Badge`].
    #[must_use]
    pub fn align_x(mut self, alignment: Alignment) -> Self {
        self.horizontal_alignment = alignment;
        self
    }

    /// Sets the vertical alignment of the content of the [`Badge`].
    #[must_use]
    pub fn align_y(mut self, alignment: Alignment) -> Self {
        self.vertical_alignment = alignment;
        self
    }

    /// Sets the height of the [`Badge`].
    #[must_use]
    pub fn height(mut self, height: Length) -> Self {
        self.height = height;
        self
    }

    /// Sets the padding of the [`Badge`].
    #[must_use]
    pub fn padding(mut self, units: u16) -> Self {
        self.padding = units;
        self
    }

    /// Sets the style of the [`Badge`].
    #[must_use]
    pub fn style(mut self, style: <Renderer::Theme as StyleSheet>::Style) -> Self {
        self.style = style;
        self
    }

    /// Sets the width of the [`Badge`].
    #[must_use]
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Badge<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let padding = self.padding.into();
        let limits = limits
            .loose()
            .width(self.width)
            .height(self.height)
            .pad(padding);

        let mut content =
            self.content
                .as_widget()
                .layout(&mut tree.children[0], renderer, &limits.loose());
        let size = limits.resolve(content.size());

        content.move_to(Point::new(padding.left, padding.top));
        content.align(self.horizontal_alignment, self.vertical_alignment, size);

        Node::with_children(size.pad(padding), vec![content])
    }

    fn on_event(
        &mut self,
        state: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) -> event::Status {
        self.content.as_widget_mut().on_event(
            &mut state.children[0],
            event,
            layout
                .children()
                .next()
                .expect("Native: Layout should have a children layout for a badge."),
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let mut children = layout.children();
        let style_sheet = theme.active(&self.style);

        let border_radius = style_sheet
            .border_radius
            .unwrap_or(bounds.height / BORDER_RADIUS_RATIO);

        renderer.fill_quad(
            renderer::Quad {
                bounds,
                border_radius: border_radius.into(),
                border_width: style_sheet.border_width,
                border_color: style_sheet.border_color.unwrap_or(Color::BLACK),
            },
            style_sheet.background,
        );

        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            &renderer::Style {
                text_color: style_sheet.text_color,
            },
            children
                .next()
                .expect("Graphics: Layout should have a children layout for Badge"),
            cursor,
            viewport,
        );
    }
}

impl<'a, Message, Renderer> From<Badge<'a, Message, Renderer>> for Element<'a, Message, Renderer>
where
    Message: 'a + Clone,
    Renderer: 'a + renderer::Renderer,
    Renderer::Theme: StyleSheet,
{
    fn from(badge: Badge<'a, Message, Renderer>) -> Self {
        Self::new(badge)
    }
}

/// The appearance of a [`Badge`](crate::native::badge::Badge).
#[derive(Clone, Copy, Debug)]
pub struct Appearance {
    /// The background of the [`Badge`](crate::native::badge::Badge).
    pub background: Background,

    /// The border radius of the [`Badge`](crate::native::badge::Badge).
    /// If no radius is specified the default one will be used.
    pub border_radius: Option<f32>,

    /// The border with of the [`Badge`](crate::native::badge::Badge).
    pub border_width: f32,

    /// The border color of the [`Badge`](crate::native::badge::Badge).
    pub border_color: Option<Color>,

    /// The default text color of the [`Badge`](crate::native::badge::Badge).
    pub text_color: Color,
}

/// The appearance of a [`Badge`](crate::native::badge::Badge).
pub trait StyleSheet {
    ///Style for the trait to use.
    type Style: Default;
    /// The normal appearance of a [`Badge`](crate::native::badge::Badge).
    fn active(&self, style: &Self::Style) -> Appearance;

    /// The appearance when the [`Badge`](crate::native::badge::Badge) is hovered.
    fn hovered(&self, style: &Self::Style) -> Appearance {
        self.active(style)
    }
}

impl std::default::Default for Appearance {
    fn default() -> Self {
        Self {
            background: Background::Color([0.87, 0.87, 0.87].into()),
            border_radius: None,
            border_width: 1.0,
            border_color: Some([0.8, 0.8, 0.8].into()),
            text_color: BLACK,
        }
    }
}

#[derive(Default)]
pub enum BadgeStyle {
    #[default]
    Default,
    Success,
    Danger,
    Warning,
    Info,
}

impl StyleSheet for Theme {
    type Style = BadgeStyle;

    fn active(&self, style: &Self::Style) -> Appearance {
        let from_colors = |color: Color, text_color: Color| Appearance {
            background: Background::Color(color),
            border_color: Some(color),
            text_color,
            ..Appearance::default()
        };

        match style {
            BadgeStyle::Default => Appearance::default(),
            BadgeStyle::Success => from_colors(GREEN, WHITE),
            BadgeStyle::Danger => from_colors(RED, WHITE),
            BadgeStyle::Warning => from_colors(ORANGE2, WHITE),
            BadgeStyle::Info => from_colors(LIGHT_BLUE, WHITE),
        }
    }
}
