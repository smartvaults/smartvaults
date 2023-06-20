// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::Row;

use crate::app::{Message, Stage};
use crate::component::Text;

#[derive(Clone)]
pub struct Breadcrumb {
    stages: Vec<Stage>,
}

impl Breadcrumb {
    pub fn new(stages: Vec<Stage>) -> Self {
        Self { stages }
    }

    pub fn view<'a>(&self) -> Row<'a, Message> {
        let mut content = Row::new().spacing(10);

        let last_index = self.stages.len().saturating_sub(1);
        for (index, stage) in self.stages.iter().enumerate() {
            content = content.push(
                Text::new(stage.to_string())
                    .on_press(Message::View(stage.clone()))
                    .smaller()
                    .extra_light()
                    .view(),
            );
            if index != last_index {
                content = content.push(Text::new(">").smaller().extra_light().view())
            }
        }

        content
    }
}
