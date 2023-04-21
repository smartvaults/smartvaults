// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Container, Row, Scrollable};
use iced::{Alignment, Element, Length};

mod generate;
mod open;
mod restore;

pub use self::generate::{GenerateMessage, GenerateState};
pub use self::open::{OpenMessage, OpenState};
pub use self::restore::{RestoreMessage, RestoreState};
use super::Message;

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
