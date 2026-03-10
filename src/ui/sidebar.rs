use iced::widget::{column, container, row, text, Space};
use iced::{Background, Element, Length};

use crate::multiplexer::pane::{PaneState, PaneStatus};
use crate::ui::theme::Theme as AppTheme;

/// Flat view-data for one workspace entry shown in the sidebar.
pub struct WorkspaceEntry {
    pub id: usize,
    pub name: String,
    pub git_branch: Option<String>,
    pub cwd: String,
    pub has_waiting: bool,
}

impl WorkspaceEntry {
    pub fn from_pane(pane: &PaneState) -> Self {
        // Use the last component of cwd as the display name.
        let name = pane
            .cwd
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string();
        Self {
            id: pane.id,
            name,
            git_branch: pane.git_branch.clone(),
            cwd: pane.cwd.to_string_lossy().into_owned(),
            has_waiting: pane.status == PaneStatus::Waiting,
        }
    }
}

/// Marker type kept for API compatibility (styling uses closures in iced 0.14).
pub struct SidebarStyle;

/// Sidebar widget displaying a list of workspace entries.
pub struct Sidebar<Message> {
    workspaces: Vec<WorkspaceEntry>,
    active_id: usize,
    _phantom: std::marker::PhantomData<Message>,
}

impl<Message: Clone + 'static> Sidebar<Message> {
    pub fn new(workspaces: &[WorkspaceEntry], active_id: usize) -> Self {
        let owned: Vec<WorkspaceEntry> = workspaces.iter().map(|ws| WorkspaceEntry {
            id: ws.id,
            name: ws.name.clone(),
            git_branch: ws.git_branch.clone(),
            cwd: ws.cwd.clone(),
            has_waiting: ws.has_waiting,
        }).collect();
        Self {
            workspaces: owned,
            active_id,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn view(&self) -> Element<'static, Message> {
        let entries: Vec<Element<'static, Message>> = self
            .workspaces
            .iter()
            .map(|ws| self.render_entry(ws))
            .collect();

        let list = column(entries).spacing(1);

        container(list)
            .width(200)
            .height(Length::Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(Background::Color(AppTheme::SIDEBAR_BG)),
                ..Default::default()
            })
            .into()
    }

    fn render_entry(&self, ws: &WorkspaceEntry) -> Element<'static, Message> {
        let is_active = ws.id == self.active_id;

        // Name row
        let name_text = text(ws.name.clone())
            .color(AppTheme::TEXT_PRIMARY)
            .size(14);

        // Optional waiting badge on the right
        let badge: Element<'static, Message> = if ws.has_waiting {
            text("●")
                .color(AppTheme::BADGE_ACTIVE)
                .size(12)
                .into()
        } else {
            Space::new().width(12).into()
        };

        let name_row = row![
            name_text,
            Space::new().width(Length::Fill),
            badge,
        ]
        .align_y(iced::Alignment::Center);

        // Optional git branch subtitle
        let branch_row: Element<'static, Message> = if let Some(branch) = &ws.git_branch {
            text(format!("  {}", branch))
                .color(AppTheme::TEXT_DIM)
                .size(11)
                .into()
        } else {
            Space::new().height(0).into()
        };

        let entry_col = column![name_row, branch_row].spacing(2);

        let bg_color = if is_active {
            AppTheme::PANE_BG
        } else {
            AppTheme::SIDEBAR_BG
        };

        container(entry_col)
            .width(Length::Fill)
            .padding([6, 10])
            .style(move |_theme| iced::widget::container::Style {
                background: Some(Background::Color(bg_color)),
                ..Default::default()
            })
            .into()
    }
}
