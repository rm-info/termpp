use termpp::ui::sidebar::{Sidebar, WorkspaceEntry, TabEntry};

fn make_tab(id: usize, name: &str, active: bool) -> TabEntry {
    TabEntry {
        id,
        name: name.to_string(),
        git_branch: if active { Some("main".to_string()) } else { None },
        terminal_title: if active { Some("Claude Code".to_string()) } else { None },
        has_waiting: false,
    }
}

#[test]
fn sidebar_renders_with_active_workspace_and_tab() {
    let entries = vec![
        WorkspaceEntry {
            id: 0,
            name: "default".to_string(),
            active_tab_id: 1,
            collapsed: false,
            tabs: vec![
                make_tab(0, "main", false),
                make_tab(1, "dev", true),
            ],
        },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        0,      // active_workspace_id
        None,   // not renaming tab
        None,   // not renaming workspace
        |_| (), // on_select_tab
        |_| (), // on_close_tab
        |_| (), // on_new_tab
        |_| (), // on_toggle_workspace
        (),     // on_new_workspace
        |_| (), // on_rename_start (tab)
        |_| (), // on_rename_change (tab)
        (),     // on_rename_commit (tab)
        (),     // on_rename_cancel (tab)
        |_| (), // on_rename_workspace_start
        |_| (), // on_rename_workspace_change
        (),     // on_rename_workspace_commit
        (),     // on_rename_workspace_cancel
        (),     // on_help
    ).view();
}

#[test]
fn sidebar_renders_collapsed_workspace() {
    let entries = vec![
        WorkspaceEntry {
            id: 0,
            name: "work".to_string(),
            active_tab_id: 0,
            collapsed: true,
            tabs: vec![make_tab(0, "main", false)],
        },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        0,
        None, None,
        |_| (), |_| (), |_| (), |_| (), (),
        |_| (), |_| (), (), (),
        |_| (), |_| (), (), (),
        (),
    ).view();
}
