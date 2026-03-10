use termpp::multiplexer::pane::detect_git_branch;

#[test]
fn detects_branch_in_git_repo() {
    let cwd = std::env::current_dir().unwrap();
    let branch = detect_git_branch(&cwd);
    assert!(branch.is_some(), "Expected a git branch in project dir");
}

#[test]
fn returns_none_outside_git_repo() {
    let tmp = std::env::temp_dir().join("not_a_git_repo_termpp_test");
    std::fs::create_dir_all(&tmp).ok();
    let branch = detect_git_branch(&tmp);
    assert!(branch.is_none());
    std::fs::remove_dir(&tmp).ok();
}
