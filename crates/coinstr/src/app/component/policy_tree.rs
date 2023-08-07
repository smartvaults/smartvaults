// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk::descriptor::policy::{PkOrF, SatisfiableItem};
use iced::widget::{Column, Row};
use iced_native::widget::Space;
use iced_native::Length;

use crate::app::Message;
use crate::component::Text;
use crate::theme::color::{CYAN, GREEN, MAGENTA};

const LEFT_SPACE: f32 = 30.0;

pub struct PolicyTree {
    item: SatisfiableItem,
}

impl PolicyTree {
    pub fn new(item: SatisfiableItem) -> Self {
        Self { item }
    }

    pub fn view(self) -> Column<'static, Message> {
        add_node(&self.item, 1)
    }
}

fn display_key(key: &PkOrF) -> String {
    match key {
        PkOrF::Pubkey(pk) => format!("<pk:{pk}>"),
        PkOrF::XOnlyPubkey(pk) => format!("<xonly-pk:{pk}>"),
        PkOrF::Fingerprint(f) => format!("<fingerprint:{f}>"),
    }
}

fn add_node(item: &SatisfiableItem, counter: usize) -> Column<'static, Message> {
    let tree = Column::new()
        .push(
            Text::new(format!("id -> {}", item.id()))
                .color(GREEN)
                .bold()
                .view(),
        )
        .push(Space::with_width(Length::Fixed(
            LEFT_SPACE / 2.0 * counter as f32,
        )));

    let mut child = Row::new().push(Space::with_width(Length::Fixed(
        LEFT_SPACE * counter as f32,
    )));

    match &item {
        SatisfiableItem::EcdsaSignature(key) => {
            child =
                child.push(Text::new(format!("{} {}", "ECDSA Sig of ", display_key(key))).view());
        }
        SatisfiableItem::SchnorrSignature(key) => {
            child =
                child.push(Text::new(format!("{} {}", "Schnorr Sig of ", display_key(key))).view());
        }
        SatisfiableItem::Sha256Preimage { hash } => {
            child = child.push(Text::new(format!("SHA256 Preimage of {hash}")).view());
        }
        SatisfiableItem::Hash256Preimage { hash } => {
            child = child.push(Text::new(format!("Double-SHA256 Preimage of {hash}")).view());
        }
        SatisfiableItem::Ripemd160Preimage { hash } => {
            child = child.push(Text::new(format!("RIPEMD160 Preimage of {hash}")).view());
        }
        SatisfiableItem::Hash160Preimage { hash } => {
            child = child.push(Text::new(format!("Double-RIPEMD160 Preimage of {hash}")).view());
        }
        SatisfiableItem::AbsoluteTimelock { value } => {
            child = child.push(Text::new(format!("{} {value}", "Absolute Timelock of ")).view());
        }
        SatisfiableItem::RelativeTimelock { value } => {
            child = child.push(Text::new(format!("{} {value}", "Relative Timelock of")).view());
        }
        SatisfiableItem::Multisig { keys, threshold } => {
            let mut child_tree = Column::new().push(
                Text::new(format!("MultiSig: {} of {}", threshold, keys.len()))
                    .color(CYAN)
                    .view(),
            );

            for x in keys.iter() {
                child_tree = child_tree.push(
                    Row::new()
                        .push(Space::with_width(Length::Fixed(
                            LEFT_SPACE * counter as f32,
                        )))
                        .push(
                            Text::new(format!("Key: {}", display_key(x)))
                                .color(MAGENTA)
                                .view(),
                        ),
                );
            }
            child = child.push(child_tree);
        }
        SatisfiableItem::Thresh { items, threshold } => {
            let mut child_tree = Column::new().push(
                Text::new(format!(
                    "Threshold Condition: {} of {}",
                    threshold,
                    items.len()
                ))
                .color(CYAN)
                .view(),
            );

            for x in items.iter() {
                child_tree = child_tree.push(
                    Row::new()
                        .push(Space::with_width(Length::Fixed(
                            LEFT_SPACE * counter as f32,
                        )))
                        .push(add_node(&x.item, counter + 1)),
                );
            }

            child = child.push(child_tree);
        }
    }

    tree.push(child)
}
