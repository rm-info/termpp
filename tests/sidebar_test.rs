use termpp::ui::sidebar::{Sidebar, WorkspaceEntry};

#[test]
fn sidebar_renders_with_active_entry() {
    // Tests that the widget tree builds without panic when there is an active entry.
    // This exercises the accent-bar code path in render_entry().
    let entries = vec![
        WorkspaceEntry { id: 0, name: "main".to_string(), git_branch: Some("main".to_string()), cwd: "/home".to_string(), has_waiting: false, terminal_title: Some("Claude Code".to_string()) },
        WorkspaceEntry { id: 1, name: "dev".to_string(),  git_branch: None, cwd: "/tmp".to_string(), has_waiting: true, terminal_title: None },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        0,     // active_id = first entry — exercises the accent-bar code path
        None,  // not renaming
        |_| (), // on_select
        |_| (), // on_close
        (),     // on_new
        |_| (), // on_rename_start
        |_| (), // on_rename_change
        (),     // on_rename_commit
        (),     // on_rename_cancel
        (),     // on_help
    ).view();
}

#[test]
fn sidebar_renders_with_no_active_match() {
    // active_id not in entries — exercises the inactive (no accent bar) path
    let entries = vec![
        WorkspaceEntry { id: 0, name: "main".to_string(), git_branch: None, cwd: "/home".to_string(), has_waiting: false, terminal_title: None },
    ];
    let _el: iced::Element<'static, ()> = Sidebar::<()>::new(
        &entries,
        99,     // active_id = no match
        None,
        |_| (), |_| (), (), |_| (), |_| (), (), (), (),
    ).view();
}
