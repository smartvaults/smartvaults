// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::{mouse, Color, Element, Length, Rectangle, Size};
use iced_futures::core::widget::Tree;

use crate::theme::color::{BLACK, TRANSPARENT};

pub struct Circle {
    radius: f32,
    color: Color,
}

impl Circle {
    pub fn new(radius: f32) -> Self {
        Self {
            radius,
            color: BLACK,
        }
    }

    pub fn color(self, color: Color) -> Self {
        Self { color, ..self }
    }
}

// pub fn circle(radius: f32) -> Circle {
// Circle::new(radius)
// }

impl<Message, Renderer> Widget<Message, Renderer> for Circle
where
    Renderer: renderer::Renderer,
{
    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(Size::new(self.radius * 2.0, self.radius * 2.0))
    }

    fn draw(
        &self,
        _state: &widget::Tree,
        renderer: &mut Renderer,
        _theme: &Renderer::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        renderer.fill_quad(
            renderer::Quad {
                bounds: layout.bounds(),
                border_radius: self.radius.into(),
                border_width: 0.0,
                border_color: TRANSPARENT,
            },
            self.color,
        );
    }
}

impl<'a, Message, Renderer> From<Circle> for Element<'a, Message, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn from(circle: Circle) -> Self {
        Self::new(circle)
    }
}
