// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use coinstr_core::util::bip::bip32::Bip32RootKey;
use iced::widget::{svg, Column, Container, Rule, Space};
use iced::Length;

mod button;

use self::button::SidebarButton;
use crate::app::{Context, Message, Stage};
use crate::component::{Icon, Text};
use crate::theme::color::DARK_RED;
use crate::theme::icon::{HOME, KEY, LOCK, SEND_PENDING, SETTING};
use crate::COINSTR_LOGO;

const MAX_WIDTH: f32 = 240.0;

#[derive(Clone, Default)]
pub struct Sidebar;

impl Sidebar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<'a>(&self, ctx: &Context) -> Container<'a, Message> {
        // Logo
        let handle = svg::Handle::from_memory(COINSTR_LOGO);
        let logo = svg(handle)
            .width(Length::Fixed(80.0))
            .height(Length::Fixed(80.0));

        // Buttons
        let home_button = SidebarButton::new("Home", Icon::new(HOME).view())
            .view(ctx, Message::View(Stage::Home));
        let policies_button = SidebarButton::new("Policies", Icon::new(KEY).view())
            .view(ctx, Message::View(Stage::Policies));
        let proposals_button = SidebarButton::new("Proposals", Icon::new(SEND_PENDING).view())
            .view(ctx, Message::View(Stage::Proposals));
        let settings_button = SidebarButton::new("Settings", Icon::new(SETTING).view())
            .view(ctx, Message::View(Stage::Setting));

        // Identity
        let fingerprint = match ctx
            .coinstr
            .keychain()
            .seed
            .fingerprint(ctx.coinstr.network())
        {
            Ok(fingerprint) => Text::new(format!("Fingerprint: {}", fingerprint)).size(22),
            Err(_) => Text::new("Fingerprint: error").color(DARK_RED),
        };
        let identity = Column::new()
            .push(fingerprint.view())
            .spacing(10)
            .padding([15, 0]);

        // Footer
        let lock_button =
            SidebarButton::new("Lock", Icon::new(LOCK).view()).view(ctx, Message::Lock);
        let version = Text::new(format!(
            "{} v{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))
        .size(16)
        .view();

        // Combine sidebar
        sidebar(
            Container::new(Column::new().push(logo).padding([30, 0]))
                .width(Length::Fill)
                .center_x(),
            Container::new(identity).width(Length::Fill).center_x(),
            sidebar_menu(vec![
                home_button,
                policies_button,
                proposals_button,
                settings_button,
            ]),
            sidebar_menu(vec![
                lock_button,
                Container::new(version).width(Length::Fill).center_x(),
            ]),
        )
    }
}

pub fn sidebar<'a, T: 'a>(
    logo: Container<'a, T>,
    identity: Container<'a, T>,
    menu: Container<'a, T>,
    footer: Container<'a, T>,
) -> Container<'a, T> {
    Container::new(
        Column::new()
            .padding(10)
            .push(logo)
            .push(Rule::horizontal(1))
            .push(identity)
            .push(Rule::horizontal(1))
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(menu.height(Length::Fill))
            .push(footer.height(Length::Shrink)),
    )
    .max_width(MAX_WIDTH)
}

pub fn sidebar_menu<'a, T: 'a>(items: Vec<Container<'a, T>>) -> Container<'a, T> {
    let mut col = Column::new().padding(15).spacing(15);
    for i in items {
        col = col.push(i)
    }
    Container::new(col)
}
