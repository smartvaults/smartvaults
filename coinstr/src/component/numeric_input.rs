// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use iced::widget::{self, text_input, Column, Row, Text};
use iced::Element;
use iced_lazy::{self, Component};

pub struct NumericInput<Message> {
    name: String,
    value: Option<u64>,
    placeholder: String,
    on_input: Option<Box<dyn Fn(Option<u64>) -> Message>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    InputChanged(String),
}

impl<Message> NumericInput<Message> {
    pub fn new<S>(name: S, value: Option<u64>) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            value,
            placeholder: String::new(),
            on_input: None,
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

    pub fn on_input(self, on_input: impl Fn(Option<u64>) -> Message + 'static) -> Self {
        Self {
            on_input: Some(Box::new(on_input)),
            ..self
        }
    }
}

impl<Message, Renderer> Component<Message, Renderer> for NumericInput<Message>
where
    Renderer: iced_native::text::Renderer + 'static,
    Renderer::Theme:
        widget::button::StyleSheet + widget::text_input::StyleSheet + widget::text::StyleSheet,
{
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Event) -> Option<Message> {
        if let Some(on_input) = &self.on_input {
            match event {
                Event::InputChanged(value) => {
                    if value.is_empty() {
                        Some((on_input)(None))
                    } else {
                        value.parse().ok().map(Some).map(on_input.as_ref())
                    }
                }
            }
        } else {
            None
        }
    }

    fn view(&self, _state: &Self::State) -> Element<Event, Renderer> {
        let mut text_input = text_input(
            &self.placeholder,
            self.value
                .as_ref()
                .map(u64::to_string)
                .as_deref()
                .unwrap_or(""),
        )
        .padding(10)
        .size(20);

        if self.on_input.is_some() {
            text_input = text_input.on_input(Event::InputChanged);
        }

        let text = Text::new(&self.name).size(20);

        Column::new()
            .push(text)
            .push(Row::new().push(text_input))
            .spacing(5)
            .into()
    }
}

impl<'a, Message, Renderer> From<NumericInput<Message>> for Element<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: 'static + iced_native::text::Renderer,
    Renderer::Theme:
        widget::button::StyleSheet + widget::text_input::StyleSheet + widget::text::StyleSheet,
{
    fn from(numeric_input: NumericInput<Message>) -> Self {
        iced_lazy::component(numeric_input)
    }
}
