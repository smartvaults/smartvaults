// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Button, Container, Row, Text};
use iced::{Alignment, Length};

use crate::app::{Context, Message};
use crate::component::button::ButtonStyle;

#[derive(Clone)]
pub struct SidebarButton<'a> {
    text: &'a str,
    icon: Text<'a>,
}

impl<'a> SidebarButton<'a> {
    pub fn new(text: &'a str, icon: Text<'a>) -> Self {
        Self { text, icon }
    }

    pub fn view(&self, ctx: &Context, msg: Message) -> Container<'a, Message> {
        let mut style = ButtonStyle::Bordered.into();

        if let Message::View(stage) = msg.clone() {
            if ctx.stage.eq(&stage) {
                style = ButtonStyle::Primary.into();
            }
        }

        let content = Container::new(
            Row::new()
                .push(self.icon.clone())
                .push(Text::new(self.text))
                .spacing(10)
                .width(Length::Fill)
                .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .center_x()
        .padding(5);

        Container::new(
            Button::new(content)
                .on_press(msg)
                .width(Length::Fill)
                .style(style),
        )
        .width(Length::Fill)
    }
}
