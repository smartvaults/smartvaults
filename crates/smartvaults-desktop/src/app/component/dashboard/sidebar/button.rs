// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::Container;
use iced::Length;

use crate::app::{Context, Message};
use crate::component::{Button, ButtonStyle};

#[derive(Clone)]
pub struct SidebarButton<'a> {
    text: &'a str,
    icon: char,
}

impl<'a> SidebarButton<'a> {
    pub fn new(text: &'a str, icon: char) -> Self {
        Self { text, icon }
    }

    pub fn view(&self, ctx: &Context, msg: Message) -> Container<'a, Message> {
        let mut style = ButtonStyle::Bordered;

        if let Message::View(stage) = msg.clone() {
            if ctx.stage.eq(&stage) {
                style = ButtonStyle::Primary;
            }
        }

        let btn = Button::new()
            .icon(self.icon)
            .text(self.text)
            .on_press(msg)
            .style(style)
            .width(Length::Fill)
            .view();

        Container::new(btn).width(Length::Fill)
    }
}
