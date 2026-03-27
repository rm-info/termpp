use iced::widget::{column, container, mouse_area, row, text, text_input, Space};
use iced::{Background, Element, Length};

use crate::multiplexer::pane::{PaneState, PaneStatus};
use crate::ui::theme::Theme as AppTheme;

pub const RENAME_INPUT_ID: &str = "sidebar_rename";

/// Flat view-data for one workspace entry shown in the sidebar.
pub struct WorkspaceEntry {
    pub id: usize,
    pub name: String,
    pub git_branch: Option<String>,
    pub cwd: String,
    pub has_waiting: bool,
    pub terminal_title: Option<String>,
}

impl WorkspaceEntry {
    pub fn from_pane(pane: &PaneState) -> Self {
        let name = pane.pane_name.clone().unwrap_or_else(|| {
            pane.cwd
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string()
        });
        Self {
            id: pane.id,
            name,
            git_branch: pane.git_branch.clone(),
            cwd: pane.cwd.to_string_lossy().into_owned(),
            has_waiting: pane.status == PaneStatus::Waiting,
            terminal_title: pane.terminal_title.clone(),
        }
    }
}

/// Sidebar widget displaying a list of workspace entries.
/// All callbacks are plain fn pointers so they are Copy + 'static.
pub struct Sidebar<Message: Clone + 'static> {
    workspaces:       Vec<WorkspaceEntry>,
    active_id:        usize,
    renaming:         Option<(usize, String)>,
    on_select:        fn(usize)  -> Message,
    on_close:         fn(usize)  -> Message,
    on_new:           Message,
    on_rename_start:  fn(usize)  -> Message,
    on_rename_change: fn(String) -> Message,
    on_rename_commit: Message,
    on_rename_cancel: Message,
    on_help:          Message,
}

impl<Message: Clone + 'static> Sidebar<Message> {
    pub fn new(
        workspaces:       &[WorkspaceEntry],
        active_id:        usize,
        renaming:         Option<(usize, String)>,
        on_select:        fn(usize)  -> Message,
        on_close:         fn(usize)  -> Message,
        on_new:           Message,
        on_rename_start:  fn(usize)  -> Message,
        on_rename_change: fn(String) -> Message,
        on_rename_commit: Message,
        on_rename_cancel: Message,
        on_help:          Message,
    ) -> Self {
        let owned = workspaces.iter().map(|ws| WorkspaceEntry {
            id: ws.id,
            name: ws.name.clone(),
            git_branch: ws.git_branch.clone(),
            cwd: ws.cwd.clone(),
            has_waiting: ws.has_waiting,
            terminal_title: ws.terminal_title.clone(),
        }).collect();
        Self {
            workspaces: owned,
            active_id,
            renaming,
            on_select,
            on_close,
            on_new,
            on_rename_start,
            on_rename_change,
            on_rename_commit,
            on_rename_cancel,
            on_help,
        }
    }

    pub fn view(&self) -> Element<'static, Message> {
        let entries: Vec<Element<'static, Message>> = self
            .workspaces
            .iter()
            .map(|ws| self.render_entry(ws))
            .collect();

        let new_msg = self.on_new.clone();
        let new_btn: Element<'static, Message> = mouse_area(
            container(text("+").color(AppTheme::TEXT_DIM).size(16))
                .width(Length::Fill)
                .padding([6, 10])
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
                    ..Default::default()
                })
        )
        .on_press(new_msg)
        .into();

        let help_msg = self.on_help.clone();
        let help_btn: Element<'static, Message> = mouse_area(
            container(text("?").color(AppTheme::TEXT_DIM).size(16))
                .width(Length::Fill)
                .padding([6, 10])
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
                    ..Default::default()
                })
        )
        .on_press(help_msg)
        .into();

        container(
            column(entries)
                .spacing(1)
                .push(new_btn)
                .push(Space::new().height(Length::Fill))
                .push(help_btn)
        )
        .width(200)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style {
            background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
            ..Default::default()
        })
        .into()
    }

    fn render_entry(&self, ws: &WorkspaceEntry) -> Element<'static, Message> {
        let is_active   = ws.id == self.active_id;
        let is_renaming = self.renaming.as_ref().map(|(id, _)| *id) == Some(ws.id);
        let bg_color    = if is_active { AppTheme::PANE_BG } else { AppTheme::SIDEBAR_BG };

        // ── Rename mode ──────────────────────────────────────────────────────
        if is_renaming {
            let value        = self.renaming.as_ref().map(|(_, s)| s.clone()).unwrap_or_default();
            let change_fn    = self.on_rename_change;
            let commit_msg   = self.on_rename_commit.clone();
            let cancel_msg   = self.on_rename_cancel.clone();

            let input: Element<'static, Message> = text_input("Name…", &value)
                .id(iced::widget::Id::new(RENAME_INPUT_ID))
                .on_input(move |s| change_fn(s))
                .on_submit(commit_msg)
                .size(13)
                .padding([2, 4])
                .into();

            let cancel_btn: Element<'static, Message> = mouse_area(
                text("×").color(AppTheme::TEXT_DIM).size(14)
            )
            .on_press(cancel_msg)
            .into();

            return container(
                row![input, cancel_btn].spacing(4).align_y(iced::Alignment::Center)
            )
            .width(Length::Fill)
            .padding([6, 10])
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(bg_color)),
                ..Default::default()
            })
            .into();
        }

        // ── Normal mode ──────────────────────────────────────────────────────
        let select_msg      = (self.on_select)(ws.id);
        let close_msg       = (self.on_close)(ws.id);
        let rename_start    = (self.on_rename_start)(ws.id);

        let name_text = text(ws.name.clone()).color(AppTheme::TEXT_PRIMARY).size(14);

        let badge: Element<'static, Message> = if ws.has_waiting {
            text("●").color(AppTheme::BADGE_ACTIVE).size(12).into()
        } else {
            Space::new().width(4).into()
        };

        let rename_btn: Element<'static, Message> = mouse_area(
            text("✎").color(AppTheme::TEXT_DIM).size(12)
        )
        .on_press(rename_start)
        .into();

        let close_btn: Element<'static, Message> = mouse_area(
            text("×").color(AppTheme::TEXT_DIM).size(14)
        )
        .on_press(close_msg)
        .into();

        let name_row = row![
            name_text,
            Space::new().width(Length::Fill),
            badge,
            rename_btn,
            close_btn,
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center);

        let branch_row: Element<'static, Message> = if let Some(branch) = &ws.git_branch {
            text(format!("  {branch}")).color(AppTheme::TEXT_DIM).size(11).into()
        } else {
            Space::new().height(0).into()
        };

        let title_row: Element<'static, Message> = if let Some(title) = &ws.terminal_title {
            text(format!("  {title}")).color(AppTheme::TEXT_DIM).size(11).into()
        } else {
            Space::new().height(0).into()
        };

        let content = container(column![name_row, branch_row, title_row].spacing(2))
            .width(Length::Fill)
            .padding([6, 10])
            .style(move |_| iced::widget::container::Style {
                background: Some(Background::Color(bg_color)),
                ..Default::default()
            });

        if is_active {
            let accent = container(Space::new())
                .width(3)
                .height(Length::Fill)
                .style(|_| iced::widget::container::Style {
                    background: Some(Background::Color(AppTheme::ACCENT)),
                    ..Default::default()
                });
            mouse_area(
                row![accent, content]
                    .width(Length::Fill)
                    .height(Length::Shrink)
            )
            .on_press(select_msg)
            .into()
        } else {
            mouse_area(content)
                .on_press(select_msg)
                .into()
        }
    }
}
