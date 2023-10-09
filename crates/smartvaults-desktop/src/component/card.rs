// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{container, Column, Container};
use iced::{theme, Background, BorderRadius, Element, Theme};

use crate::theme::color::WHITE;

pub struct Card<Message> {
    head: Element<'static, Message>,
    body: Element<'static, Message>,
    foot: Option<Element<'static, Message>>,
    max_width: f32,
}

impl<Message> Card<Message>
where
    Message: Clone + 'static,
{
    pub fn new<H, B>(head: H, body: B) -> Self
    where
        H: Into<Element<'static, Message>>,
        B: Into<Element<'static, Message>>,
    {
        Self {
            head: head.into(),
            body: body.into(),
            foot: None,
            max_width: u32::MAX as f32,
        }
    }

    pub fn foot<F>(mut self, foot: F) -> Self
    where
        F: Into<Element<'static, Message>>,
    {
        self.foot = Some(foot.into());
        self
    }

    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = width;
        self
    }

    pub fn view(self) -> Element<'static, Message> {
        let mut card = Column::new().push(self.head).push(self.body);

        if let Some(foot) = self.foot {
            card = card.push(foot);
        }

        card = card.max_width(self.max_width);

        Container::new(card)
            .padding(10.0)
            .style(CardStyle::Primary)
            .into()
    }
}

#[derive(Default, Clone, Copy)]
pub enum CardStyle {
    #[default]
    Primary,
}

impl container::StyleSheet for CardStyle {
    type Style = Theme;

    fn appearance(&self, style: &Self::Style) -> container::Appearance {
        let palette = style.palette();
        match self {
            Self::Primary => container::Appearance {
                text_color: None,
                background: Some(Background::Color(palette.background)),
                border_radius: BorderRadius::from(10.0),
                border_width: 1.0,
                border_color: WHITE,
            },
        }
    }
}

impl From<CardStyle> for theme::Container {
    fn from(style: CardStyle) -> Self {
        theme::Container::Custom(Box::new(style))
    }
}
