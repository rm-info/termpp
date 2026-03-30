use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::multiplexer::layout::{Layout, PaneId};
use crate::multiplexer::pane::PaneState;
use crate::terminal::emulator::Emulator;

pub type WorkspaceId = usize;
pub type TabId = usize;

pub struct Tab {
    pub id: TabId,
    pub name: String,
    pub layout: Layout,
    pub panes: HashMap<PaneId, PaneState>,
    pub emulators: HashMap<PaneId, Arc<Mutex<Emulator>>>,
    pub active_pane: PaneId,
    pub next_pane_id: usize,
    pub last_output_counts: HashMap<PaneId, u64>,
}

pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub tabs: Vec<Tab>,
    pub active_tab: TabId,
    pub collapsed: bool,
}

impl Workspace {
    /// Index of the active tab in `self.tabs`. Falls back to 0 if active_tab id not found.
    pub fn active_tab_idx(&self) -> usize {
        self.tabs.iter().position(|t| t.id == self.active_tab).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplexer::layout::Layout;
    use std::collections::HashMap;

    fn make_tab(id: usize) -> Tab {
        Tab {
            id,
            name: format!("tab-{id}"),
            layout: Layout::new(0),
            panes: HashMap::new(),
            emulators: HashMap::new(),
            active_pane: 0,
            next_pane_id: 1,
            last_output_counts: HashMap::new(),
        }
    }

    #[test]
    fn workspace_active_tab_idx_finds_middle_tab() {
        let ws = Workspace {
            id: 0,
            name: "test".into(),
            tabs: vec![make_tab(0), make_tab(5), make_tab(10)],
            active_tab: 5,
            collapsed: false,
        };
        assert_eq!(ws.active_tab_idx(), 1);
    }

    #[test]
    fn workspace_active_tab_idx_defaults_to_zero_on_miss() {
        let ws = Workspace {
            id: 0,
            name: "test".into(),
            tabs: vec![make_tab(0)],
            active_tab: 99,
            collapsed: false,
        };
        assert_eq!(ws.active_tab_idx(), 0);
    }
}
