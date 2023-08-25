// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{svg, Column, Container, Space};
use iced::Length;

mod button;

use self::button::SidebarButton;
use crate::app::{Context, Message, Stage};
use crate::component::Text;
use crate::constants::APP_LOGO;
use crate::theme::icon::{
    CONTACTS, HISTORY, HOME, KEY, LINK, LIST, LOCK, SEND_PENDING, SETTING, WALLET,
};

const MAX_WIDTH: f32 = 240.0;

#[derive(Clone, Default)]
pub struct Sidebar;

impl Sidebar {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&self, ctx: &Context) -> Container<'a, Message> {
        // Logo
        let handle = svg::Handle::from_memory(APP_LOGO);
        let logo = svg(handle)
            .width(Length::Fixed(100.0))
            .height(Length::Fixed(100.0));

        // Buttons
        let home_button =
            SidebarButton::new("Dashboard", HOME).view(ctx, Message::View(Stage::Dashboard));
        let policies_button =
            SidebarButton::new("Policies", WALLET).view(ctx, Message::View(Stage::Policies));
        let proposals_button = SidebarButton::new("Proposals", SEND_PENDING)
            .view(ctx, Message::View(Stage::Proposals));
        let history_button =
            SidebarButton::new("History", HISTORY).view(ctx, Message::View(Stage::History));
        let addresses_button =
            SidebarButton::new("Addresses", LIST).view(ctx, Message::View(Stage::Addresses(None)));
        let signers_button =
            SidebarButton::new("Signers", KEY).view(ctx, Message::View(Stage::Signers));
        let contacts_button =
            SidebarButton::new("Contacts", CONTACTS).view(ctx, Message::View(Stage::Contacts));
        let connect_button =
            SidebarButton::new("Connect", LINK).view(ctx, Message::View(Stage::NostrConnect));
        let settings_button =
            SidebarButton::new("Settings", SETTING).view(ctx, Message::View(Stage::Settings));

        // Footer
        let lock_button = SidebarButton::new("Lock", LOCK).view(ctx, Message::Lock);
        let version = Text::new(format!(
            "{} v{} ({})",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            env!("GIT_HASH").chars().take(8).collect::<String>(),
        ))
        .smaller()
        .view();

        // Combine sidebar
        sidebar(
            Container::new(Column::new().push(logo).padding([30, 0]))
                .width(Length::Fill)
                .center_x(),
            sidebar_menu(vec![
                home_button,
                policies_button,
                proposals_button,
                history_button,
                addresses_button,
                signers_button,
                contacts_button,
                connect_button,
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
    menu: Container<'a, T>,
    footer: Container<'a, T>,
) -> Container<'a, T> {
    Container::new(
        Column::new()
            .padding(10)
            .push(logo)
            .push(Space::with_height(Length::Fixed(10.0)))
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
