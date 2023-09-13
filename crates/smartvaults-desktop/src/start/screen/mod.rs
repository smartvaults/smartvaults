// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Container, Row, Scrollable};
use iced::{Alignment, Element, Length};

mod generate;
mod open;
mod restore;
mod setting;

pub use self::generate::{GenerateMessage, GenerateState};
pub use self::open::{OpenMessage, OpenState};
pub use self::restore::{RestoreMessage, RestoreState};
pub use self::setting::{SettingMessage, SettingState};
use super::{Message, Stage};
use crate::component::{Button, ButtonStyle};
use crate::theme::icon::SETTING;

fn view(column: Column<Message>) -> Element<Message> {
    let content = Container::new(
        column
            .align_items(Alignment::Center)
            .spacing(20)
            .padding(20)
            .max_width(400),
    )
    .width(Length::Fill)
    .center_x()
    .center_y();

    Column::new()
        .push(
            Row::new()
                .push(Row::new().width(Length::Fill))
                .push(
                    Button::new()
                        .icon(SETTING)
                        .style(ButtonStyle::Transparent { text_color: None })
                        .on_press(Message::View(Stage::Setting))
                        .width(Length::Fixed(40.0))
                        .view(),
                )
                .padding(10)
                .align_items(Alignment::Center),
        )
        .push(
            Row::new().push(
                Container::new(Scrollable::new(content))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x()
                    .center_y(),
            ),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
