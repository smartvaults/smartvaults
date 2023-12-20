// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::Row;
use iced::{Alignment, Color};
use smartvaults_sdk::core::bitcoin;

use crate::component::Text;
use crate::constants::{BIGGER_FONT_SIZE, BIG_FONT_SIZE, DEFAULT_FONT_SIZE};
use crate::theme::color::{GREEN, GREY1, RED};

const PREFIXES: [&str; 6] = ["0.00", "0.0", "0.", "000", "00", "0"];

/// Amount sign
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountSign {
    Positive,
    Negative,
}

pub struct Amount {
    amount: bitcoin::Amount,
    sign: Option<AmountSign>,
    color: Option<Color>,
    size: u16,
    bold: bool,
    hidden: bool,
}

impl Amount {
    pub fn new(amount: u64) -> Self {
        Self {
            amount: bitcoin::Amount::from_sat(amount),
            sign: None,
            color: None,
            size: DEFAULT_FONT_SIZE,
            bold: false,
            hidden: false,
        }
    }

    pub fn sign(mut self, sign: AmountSign) -> Self {
        self.color = Some(match sign {
            AmountSign::Positive => GREEN,
            AmountSign::Negative => RED,
        });
        self.sign = Some(sign);
        self
    }

    pub fn override_color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    pub fn big(self) -> Self {
        self.size(BIG_FONT_SIZE)
    }

    pub fn bigger(self) -> Self {
        self.size(BIGGER_FONT_SIZE)
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn view<T>(self) -> Row<'static, T>
    where
        T: Clone + 'static,
    {
        let spacing: u16 = if self.size >= BIGGER_FONT_SIZE { 10 } else { 5 };

        let row = if self.hidden {
            Row::new()
                .spacing(spacing)
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
                .push(Text::new("*").size(self.size).view())
        } else {
            let btc: String = format!("{:.8}", self.amount.to_btc());
            Row::new()
                .spacing(spacing)
                .push(split_digits(
                    btc[0..btc.len() - 6].to_string(),
                    self.size,
                    self.bold,
                    self.color,
                ))
                .push(if self.amount.to_sat() < 1_000_000 {
                    split_digits(
                        btc[btc.len() - 6..btc.len() - 3].to_string(),
                        self.size,
                        self.bold,
                        self.color,
                    )
                } else {
                    Row::new().push(
                        Text::new(btc[btc.len() - 6..btc.len() - 3].to_string())
                            .bold_maybe(self.bold)
                            .size(self.size)
                            .view(),
                    )
                })
                .push(if self.amount.to_sat() < 1000 {
                    split_digits(
                        btc[btc.len() - 3..btc.len()].to_string(),
                        self.size,
                        self.bold,
                        self.color,
                    )
                } else {
                    Row::new().push(
                        Text::new(btc[btc.len() - 3..btc.len()].to_string())
                            .bold_maybe(self.bold)
                            .size(self.size)
                            .color_maybe(self.color)
                            .view(),
                    )
                })
        };

        let mut items = Vec::with_capacity(usize::from(self.sign.is_some() && !self.hidden) + 2);

        if !self.hidden {
            if let Some(sign) = self.sign {
                items.push(
                    Text::new(match sign {
                        AmountSign::Positive => "+",
                        AmountSign::Negative => "-",
                    })
                    .color_maybe(self.color)
                    .bold()
                    .size(self.size)
                    .view(),
                );
            }
        }

        items.push(row.into());
        items.push(Text::new("BTC").size(self.size).color(GREY1).view());

        Row::with_children(items)
            .spacing(spacing)
            .align_items(Alignment::Center)
    }
}

fn split_digits<'a, T>(mut s: String, size: u16, bold: bool, color: Option<Color>) -> Row<'a, T>
where
    T: Clone + 'static,
{
    for prefix in PREFIXES.into_iter() {
        if s.starts_with(prefix) {
            let right = s.split_off(prefix.len());
            let mut row = Row::new().push(Text::new(s).size(size).color(GREY1).view());

            if !right.is_empty() {
                let text = if bold {
                    Text::new(right).bold().size(size).color_maybe(color)
                } else {
                    Text::new(right).size(size).color_maybe(color)
                };
                row = row.push(text.view());
            };

            return row;
        }
    }

    if bold {
        Row::new().push(Text::new(s).bold().size(size).color_maybe(color).view())
    } else {
        Row::new().push(Text::new(s).size(size).color_maybe(color).view())
    }
}
