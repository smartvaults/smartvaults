// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{svg, Column, Container, PickList, Space};
use iced::Length;

mod button;

use self::button::SidebarButton;
use crate::app::context::{Mode, AVAILABLE_MODES};
use crate::app::{Context, Message, Stage};
use crate::component::{rule, Text};
use crate::constants::{APP_LOGO, APP_NAME};
use crate::theme::icon::{CONTACTS, HISTORY, HOME, KEY, LINK, LIST, LOCK, PEOPLE, SETTING, VAULT};

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

        // Dropdown
        let mode_selector = PickList::new(
            AVAILABLE_MODES.to_vec(),
            Some(ctx.mode),
            Message::ChangeMode,
        )
        .width(Length::Fill)
        .padding(10);

        // Buttons
        let home_button =
            SidebarButton::new("Dashboard", HOME).view(ctx, Message::View(Stage::Dashboard));
        let vaults_button =
            SidebarButton::new("Vaults", VAULT).view(ctx, Message::View(Stage::Vaults));
        let history_button =
            SidebarButton::new("History", HISTORY).view(ctx, Message::View(Stage::History));
        let addresses_button =
            SidebarButton::new("Addresses", LIST).view(ctx, Message::View(Stage::Addresses(None)));
        let signers_button =
            SidebarButton::new("Signers", KEY).view(ctx, Message::View(Stage::Signers));
        let key_agents_button =
            SidebarButton::new("Key Agents", PEOPLE).view(ctx, Message::View(Stage::KeyAgents));
        let contacts_button =
            SidebarButton::new("Contacts", CONTACTS).view(ctx, Message::View(Stage::Contacts));
        let connect_button =
            SidebarButton::new("Connect", LINK).view(ctx, Message::View(Stage::NostrConnect));
        let settings_button =
            SidebarButton::new("Settings", SETTING).view(ctx, Message::View(Stage::Settings));

        // Footer
        let lock_button = SidebarButton::new("Lock", LOCK).view(ctx, Message::Lock);
        let version = Text::new(format!(
            "{APP_NAME} v{} ({})",
            env!("CARGO_PKG_VERSION"),
            smartvaults_sdk::git_hash_version()
                .chars()
                .take(8)
                .collect::<String>(),
        ))
        .smaller()
        .view();

        // Combine sidebar
        sidebar(
            Container::new(Column::new().push(logo).padding([30, 0]))
                .width(Length::Fill)
                .center_x(),
            mode_selector,
            sidebar_menu(vec![
                home_button,
                vaults_button,
                history_button,
                addresses_button,
                signers_button,
                key_agents_button,
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
    selector: PickList<'a, Mode, T>,
    menu: Container<'a, T>,
    footer: Container<'a, T>,
) -> Container<'a, T> {
    Container::new(
        Column::new()
            .padding(10)
            .push(logo)
            .push(Space::with_height(Length::Fixed(10.0)))
            .push(rule::horizontal())
            .push(Column::new().padding(15).push(selector))
            .push(rule::horizontal())
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
