use iced::widget::{column, container, mouse_area, row, text, text_input, Space};
use iced::{Background, Element, Length};

use crate::ui::theme::Theme as AppTheme;

pub const RENAME_INPUT_ID:    &str = "sidebar_rename";
pub const RENAME_WS_INPUT_ID: &str = "sidebar_rename_ws";

pub struct TabEntry {
    pub id: usize,
    pub name: String,
    pub git_branch: Option<String>,
    pub terminal_title: Option<String>,
    pub has_waiting: bool,
}

pub struct WorkspaceEntry {
    pub id: usize,
    pub name: String,
    pub tabs: Vec<TabEntry>,
    pub active_tab_id: usize,
    pub collapsed: bool,
}

pub struct Sidebar<Message: Clone + 'static> {
    workspaces:                  Vec<WorkspaceEntry>,
    active_workspace_id:         usize,
    renaming:                    Option<(usize, String)>, // (tab_id, current_name)
    renaming_workspace:          Option<(usize, String)>, // (ws_id, current_name)
    on_select_tab:               fn(usize) -> Message,
    on_close_tab:                fn(usize) -> Message,
    on_new_tab:                  fn(usize) -> Message,    // arg = workspace_id
    on_toggle_workspace:         fn(usize) -> Message,
    on_new_workspace:            Message,
    on_rename_start:             fn(usize) -> Message,    // tab rename (double-click / Ctrl+Shift+R)
    on_rename_change:            fn(String) -> Message,
    on_rename_commit:            Message,
    on_rename_cancel:            Message,
    on_rename_workspace_start:   fn(usize) -> Message,    // workspace rename (double-click)
    on_rename_workspace_change:  fn(String) -> Message,
    on_rename_workspace_commit:  Message,
    on_rename_workspace_cancel:  Message,
    on_help:                     Message,
}

impl<Message: Clone + 'static> Sidebar<Message> {
    pub fn new(
        workspaces:                  &[WorkspaceEntry],
        active_workspace_id:         usize,
        renaming:                    Option<(usize, String)>,
        renaming_workspace:          Option<(usize, String)>,
        on_select_tab:               fn(usize) -> Message,
        on_close_tab:                fn(usize) -> Message,
        on_new_tab:                  fn(usize) -> Message,
        on_toggle_workspace:         fn(usize) -> Message,
        on_new_workspace:            Message,
        on_rename_start:             fn(usize) -> Message,
        on_rename_change:            fn(String) -> Message,
        on_rename_commit:            Message,
        on_rename_cancel:            Message,
        on_rename_workspace_start:   fn(usize) -> Message,
        on_rename_workspace_change:  fn(String) -> Message,
        on_rename_workspace_commit:  Message,
        on_rename_workspace_cancel:  Message,
        on_help:                     Message,
    ) -> Self {
        let owned = workspaces.iter().map(|ws| WorkspaceEntry {
            id: ws.id,
            name: ws.name.clone(),
            active_tab_id: ws.active_tab_id,
            collapsed: ws.collapsed,
            tabs: ws.tabs.iter().map(|t| TabEntry {
                id: t.id,
                name: t.name.clone(),
                git_branch: t.git_branch.clone(),
                terminal_title: t.terminal_title.clone(),
                has_waiting: t.has_waiting,
            }).collect(),
        }).collect();
        Self {
            workspaces: owned,
            active_workspace_id,
            renaming,
            renaming_workspace,
            on_select_tab,
            on_close_tab,
            on_new_tab,
            on_toggle_workspace,
            on_new_workspace,
            on_rename_start,
            on_rename_change,
            on_rename_commit,
            on_rename_cancel,
            on_rename_workspace_start,
            on_rename_workspace_change,
            on_rename_workspace_commit,
            on_rename_workspace_cancel,
            on_help,
        }
    }

    pub fn view(&self) -> Element<'static, Message> {
        // Header: "WORKSPACES" label + [+] new workspace + [?] help
        let new_ws_msg = self.on_new_workspace.clone();
        let help_msg   = self.on_help.clone();

        let header: Element<'static, Message> = container(
            row![
                text("WORKSPACES")
                    .color(AppTheme::TEXT_DIM)
                    .size(10),
                Space::new().width(Length::Fill),
                mouse_area(text("+").color(AppTheme::TEXT_DIM).size(14))
                    .on_press(new_ws_msg),
                mouse_area(text("?").color(AppTheme::TEXT_DIM).size(14))
                    .on_press(help_msg),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding([6, 10])
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into();

        let mut items: Vec<Element<'static, Message>> = vec![header];

        for ws in &self.workspaces {
            items.push(self.render_workspace(ws));
            if !ws.collapsed {
                let ws_is_active = ws.id == self.active_workspace_id;
                for tab in &ws.tabs {
                    // A tab is "active" (full accent) only if its workspace is also active
                    let is_active = ws_is_active && ws.active_tab_id == tab.id;
                    items.push(self.render_tab(tab, ws.id, is_active));
                }
            }
        }

        container(
            column(items)
                .spacing(0)
                .push(Space::new().height(Length::Fill))
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into()
    }

    fn render_workspace(&self, ws: &WorkspaceEntry) -> Element<'static, Message> {
        let is_renaming_this = self.renaming_workspace.as_ref().map(|(id, _)| *id) == Some(ws.id);

        if is_renaming_this {
            let value      = self.renaming_workspace.as_ref().map(|(_, s)| s.clone()).unwrap_or_default();
            let change_fn  = self.on_rename_workspace_change;
            let commit_msg = self.on_rename_workspace_commit.clone();
            let cancel_msg = self.on_rename_workspace_cancel.clone();

            let input: Element<'static, Message> = text_input("Nom…", &value)
                .id(iced::widget::Id::new(RENAME_WS_INPUT_ID))
                .on_input(move |s| change_fn(s))
                .on_submit(commit_msg)
                .size(12)
                .padding([2, 4])
                .into();

            let cancel: Element<'static, Message> = mouse_area(
                text("×").color(AppTheme::TEXT_DIM).size(13)
            )
            .on_press(cancel_msg)
            .into();

            return container(
                row![Space::new().width(3), input, cancel]
                    .spacing(4)
                    .align_y(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .padding([5, 8])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(AppTheme::PANE_BG)),
                ..Default::default()
            })
            .into();
        }

        let is_active   = ws.id == self.active_workspace_id;
        let arrow       = if ws.collapsed { "▸" } else { "▾" };
        let toggle_msg  = (self.on_toggle_workspace)(ws.id);
        let rename_msg  = (self.on_rename_workspace_start)(ws.id);
        let new_tab_msg = (self.on_new_tab)(ws.id);

        let (accent_color, text_color, bg_color) = if is_active {
            (AppTheme::ACCENT_WS, AppTheme::TEXT_PRIMARY, AppTheme::PANE_BG)
        } else {
            (iced::Color::TRANSPARENT, AppTheme::TEXT_DIM, AppTheme::SIDEBAR_BG)
        };

        let accent = container(Space::new())
            .width(3)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(accent_color)),
                ..Default::default()
            });

        let content = container(
            row![
                text(arrow).color(if is_active { AppTheme::ACCENT_WS } else { AppTheme::TEXT_DIM }).size(10),
                text(ws.name.clone()).color(text_color).size(12),
                Space::new().width(Length::Fill),
                mouse_area(text("+").color(AppTheme::TEXT_DIM).size(12))
                    .on_press(new_tab_msg),
            ]
            .spacing(5)
            .align_y(iced::Alignment::Center)
        )
        .width(Length::Fill)
        .padding([5, 8])
        .style(move |_| iced::widget::container::Style {
            background: Some(Background::Color(bg_color)),
            ..Default::default()
        });

        mouse_area(
            row![accent, content].width(Length::Fill).height(Length::Shrink)
        )
        .on_press(toggle_msg)
        .on_double_click(rename_msg)
        .into()
    }

    fn render_tab(
        &self,
        tab: &TabEntry,
        workspace_id: usize,
        is_active: bool,
    ) -> Element<'static, Message> {
        let is_renaming = self.renaming.as_ref().map(|(id, _)| *id) == Some(tab.id);

        if is_renaming {
            let value      = self.renaming.as_ref().map(|(_, s)| s.clone()).unwrap_or_default();
            let change_fn  = self.on_rename_change;
            let commit_msg = self.on_rename_commit.clone();
            let cancel_msg = self.on_rename_cancel.clone();

            let input: Element<'static, Message> = text_input("Name…", &value)
                .id(iced::widget::Id::new(RENAME_INPUT_ID))
                .on_input(move |s| change_fn(s))
                .on_submit(commit_msg)
                .size(12)
                .padding([2, 4])
                .into();

            let cancel: Element<'static, Message> = mouse_area(
                text("×").color(AppTheme::TEXT_DIM).size(13)
            )
            .on_press(cancel_msg)
            .into();

            return container(
                row![
                    Space::new().width(17), // 3px accent + 14px indent placeholder
                    input,
                    cancel,
                ]
                .spacing(4)
                .align_y(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .padding([4, 6])
            .style(|_| iced::widget::container::Style {
                background: Some(Background::Color(AppTheme::PANE_BG)),
                ..Default::default()
            })
            .into();
        }

        let select_msg = (self.on_select_tab)(tab.id);
        let close_msg  = (self.on_close_tab)(tab.id);
        let rename_dbl = (self.on_rename_start)(tab.id);

        let (accent_color, name_color, bg_color) = if is_active {
            (AppTheme::ACCENT, AppTheme::TEXT_PRIMARY, AppTheme::PANE_BG)
        } else {
            (iced::Color::TRANSPARENT, AppTheme::TEXT_DIM, AppTheme::SIDEBAR_BG)
        };

        // Left-side: 14px workspace-level indent spacer + 3px tab accent
        let indent  = Space::new().width(14);
        let accent_bar = container(Space::new())
            .width(3)
            .height(Length::Fill)
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(accent_color)),
                ..Default::default()
            });

        let badge: Element<'static, Message> = if tab.has_waiting {
            text("●").color(AppTheme::BADGE_ACTIVE).size(10).into()
        } else {
            Space::new().width(4).into()
        };

        let close_btn: Element<'static, Message> =
            mouse_area(text("×").color(AppTheme::TEXT_DIM).size(13))
                .on_press(close_msg)
                .into();

        let name_row = row![
            text(tab.name.clone()).color(name_color).size(13),
            Space::new().width(Length::Fill),
            badge,
            close_btn,
        ]
        .spacing(3)
        .align_y(iced::Alignment::Center);

        let branch_row: Element<'static, Message> = if let Some(b) = &tab.git_branch {
            text(format!("  {b}")).color(AppTheme::TEXT_DIM).size(11).into()
        } else {
            Space::new().height(0).into()
        };

        let title_row: Element<'static, Message> = if let Some(t) = &tab.terminal_title {
            text(format!("  {t}")).color(AppTheme::TEXT_DIM).size(10).into()
        } else {
            Space::new().height(0).into()
        };

        let content = container(column![name_row, branch_row, title_row].spacing(2))
            .width(Length::Fill)
            .padding([4, 8])
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(bg_color)),
                ..Default::default()
            });

        let _ = workspace_id; // available for future use

        mouse_area(
            row![indent, accent_bar, content].width(Length::Fill).height(Length::Shrink)
        )
        .on_press(select_msg)
        .on_double_click(rename_dbl)
        .into()
    }
}
