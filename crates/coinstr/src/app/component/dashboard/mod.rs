// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Container, Row, Rule, Scrollable};
use iced::{Element, Length};

use crate::app::{Context, Message};
use crate::component::Text;

mod navbar;
mod sidebar;

use self::navbar::Navbar;
use self::sidebar::Sidebar;

#[derive(Clone, Default)]
pub struct Dashboard {
    loaded: bool,
}

impl Dashboard {
    pub fn new() -> Self {
        Self { loaded: true }
    }

    #[allow(clippy::needless_update)]
    pub fn loaded(self, loaded: bool) -> Self {
        Self { loaded, ..self }
    }

    pub fn view<'a, T>(
        &self,
        ctx: &Context,
        content: T,
        center_x: bool,
        center_y: bool,
    ) -> Element<'a, Message>
    where
        T: Into<Element<'a, Message>>,
    {
        let mut content = Container::new(Scrollable::new(content))
            .width(Length::Fill)
            .height(Length::Fill);

        if center_x {
            content = content.center_x();
        }

        if center_y {
            content = content.center_y();
        }

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
                        Column::new()
                            .push(Navbar::new().view(ctx))
                            .push(if self.loaded {
                                content
                            } else {
                                Container::new(Text::new("Loading...").view())
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .center_x()
                                    .center_y()
                            }),
                    ),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}
