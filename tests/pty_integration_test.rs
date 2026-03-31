// Run with: cargo test --test pty_integration_test -- --ignored
use termpp::terminal::emulator::Emulator;
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn emulator_captures_echo_output() {
    let cwd = std::path::Path::new(".");
    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let mut emu = Emulator::start(80, 24, shell, cwd).expect("spawn failed");

    #[cfg(windows)]
    emu.write_input(b"echo hello\r\n").unwrap();
    #[cfg(not(windows))]
    emu.write_input(b"echo hello\n").unwrap();

    // 800ms to accommodate Windows PTY slowness
    tokio::time::sleep(Duration::from_millis(800)).await;

    let grid = emu.grid();
    let found = (0..grid.rows()).any(|row| {
        let line: String = (0..grid.cols()).map(|col| grid.cell(col, row).ch).collect();
        line.contains("hello")
    });
    assert!(found, "Expected 'hello' in grid. If test fails on slow machine, increase delay.");
}
