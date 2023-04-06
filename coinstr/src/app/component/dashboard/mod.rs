// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use iced::widget::{Column, Container, Row, Rule, Scrollable};
use iced::{Element, Length};

use crate::app::{Context, Message};

mod sidebar;

use self::sidebar::Sidebar;

#[derive(Clone, Default)]
pub struct Dashboard;

impl Dashboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<'a, T>(&self, ctx: &Context, content: T) -> Element<'a, Message>
    where
        T: Into<Element<'a, Message>>,
    {
        Column::new()
            .push(
                Row::new()
                    .push(
                        Sidebar::new()
                            .view(ctx)
                            .width(Length::Shrink)
                            .height(Length::Fill),
                    )
                    .push(Rule::vertical(1))
                    .push(
                        Column::new().push(
                            Container::new(Scrollable::new(content))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .center_x()
                                .center_y(),
                        ),
                    ),
            )
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }
}
