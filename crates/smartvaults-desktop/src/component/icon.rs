// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::alignment::Horizontal;
use iced::widget::{component, Component, Text};
use iced::{Color, Element, Length, Renderer};

use crate::constants::{BIG_ICON_SIZE, DEFAULT_ICON_SIZE};
use crate::theme::font::ICON_FONT;

#[derive(Debug, Clone)]
pub struct Icon {
    unicode: char,
    size: u16,
    width: Length,
    color: Option<Color>,
}

impl Icon {
    pub fn new(unicode: char) -> Self {
        Self {
            unicode,
            size: DEFAULT_ICON_SIZE,
            width: Length::Fixed(DEFAULT_ICON_SIZE as f32),
            color: None,
        }
    }

    pub fn size(self, size: u16) -> Self {
        Self { size, ..self }
    }

    pub fn big(self) -> Self {
        Self {
            size: BIG_ICON_SIZE,
            ..self
        }
    }

    pub fn width(self, width: Length) -> Self {
        Self { width, ..self }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }
}

impl<Message> Component<Message, Renderer> for Icon {
    type Event = ();
    type State = ();

    fn update(&mut self, _state: &mut Self::State, _event: Self::Event) -> Option<Message> {
        None
    }

    fn view(&self, _state: &Self::State) -> Element<Self::Event, Renderer> {
        let mut icon = Text::new(self.unicode.to_string())
            .font(ICON_FONT)
            .width(self.width)
            .horizontal_alignment(Horizontal::Center)
            .size(self.size);

        if let Some(color) = self.color {
            icon = icon.style(color);
        }

        icon.into()
    }
}

impl<'a, Message> From<Icon> for Element<'a, Message, Renderer>
where
    Message: 'a,
{
    fn from(icon: Icon) -> Self {
        component(icon)
    }
}
