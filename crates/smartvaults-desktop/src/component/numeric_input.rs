// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::ops::Add;
use std::str::FromStr;

use iced::widget::{component, text_input, Column, Component, Row, Text};
use iced::{Element, Length, Renderer};

pub trait Number: Add + ToString + FromStr {}

impl Number for u8 {}
impl Number for u16 {}
impl Number for u32 {}
impl Number for u64 {}
impl Number for usize {}
impl Number for f32 {}

pub struct NumericInput<T, Message>
where
    T: Number,
{
    name: String,
    value: Option<T>,
    placeholder: String,
    width: Option<Length>,
    on_input: Option<Box<dyn Fn(Option<T>) -> Message>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    InputChanged(String),
}

impl<T, Message> NumericInput<T, Message>
where
    T: Number,
{
    pub fn new<S>(name: S, value: Option<T>) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            value,
            placeholder: String::new(),
            width: None,
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

    pub fn on_input(self, on_input: impl Fn(Option<T>) -> Message + 'static) -> Self {
        Self {
            on_input: Some(Box::new(on_input)),
            ..self
        }
    }

    pub fn width(self, length: Length) -> Self {
        Self {
            width: Some(length),
            ..self
        }
    }
}

impl<T, Message> Component<Message, Renderer> for NumericInput<T, Message>
where
    T: Number,
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
                .map(T::to_string)
                .as_deref()
                .unwrap_or(""),
        )
        .padding(10);

        if self.on_input.is_some() {
            text_input = text_input.on_input(Event::InputChanged);
        }

        let mut content = Column::new();

        if !self.name.is_empty() {
            content = content.push(Text::new(&self.name));
        }

        if let Some(width) = self.width {
            content = content.width(width);
        }

        content.push(Row::new().push(text_input)).spacing(5).into()
    }
}

impl<'a, T, Message> From<NumericInput<T, Message>> for Element<'a, Message, Renderer>
where
    T: Number + 'a,
    Message: 'a,
{
    fn from(numeric_input: NumericInput<T, Message>) -> Self {
        component(numeric_input)
    }
}
