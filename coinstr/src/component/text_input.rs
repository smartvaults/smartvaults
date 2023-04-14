// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Row, TextInput as NativeTextInput};

use super::Text;

pub struct TextInput<Message> {
    name: String,
    value: String,
    placeholder: String,
    password: bool,
    on_input: Option<Box<dyn Fn(String) -> Message>>,
    on_submit: Option<Message>,
}

impl<Message> TextInput<Message>
where
    Message: Clone + 'static,
{
    pub fn new<S>(name: S, value: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            value: value.into(),
            placeholder: String::new(),
            password: false,
            on_input: None,
            on_submit: None,
        }
    }

    pub fn placeholder<S>(self, placeholder: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            placeholder: placeholder.into(),
            ..self
        }
    }

    pub fn password(self) -> Self {
        Self {
            password: true,
            ..self
        }
    }

    pub fn on_input(self, on_input: impl Fn(String) -> Message + 'static) -> Self {
        Self {
            on_input: Some(Box::new(on_input)),
            ..self
        }
    }

    pub fn on_submit(self, message: Message) -> Self {
        Self {
            on_submit: Some(message),
            ..self
        }
    }

    pub fn view(self) -> Column<'static, Message> {
        let mut text_input = NativeTextInput::new(self.placeholder.as_str(), self.value.as_str())
            .padding(10)
            .size(20);

        if self.password {
            text_input = text_input.password();
        }

        if let Some(on_input) = self.on_input {
            text_input = text_input.on_input(on_input);
        }

        if let Some(message) = self.on_submit {
            text_input = text_input.on_submit(message);
        }

        Column::new()
            .push(Row::new().push(Text::new(self.name).view()))
            .push(Row::new().push(text_input))
            .spacing(5)
    }
}
