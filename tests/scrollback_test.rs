use termpp::terminal::grid::{GridPerformer, TermEvent};
use vte::Perform;

fn make_grid(cols: usize, rows: usize) -> GridPerformer {
    let (tx, _rx) = tokio::sync::mpsc::channel::<TermEvent>(32);
    GridPerformer::new(cols, rows, tx)
}

#[test]
fn scroll_up_pushes_to_scrollback() {
    let mut g = make_grid(4, 2);
    g.print('A'); g.execute(0x0A);
    g.print('B'); g.execute(0x0A); // 'A' row displaced into scrollback
    assert_eq!(g.scrollback_len(), 1);
}

#[test]
fn visible_row_at_offset_zero_returns_current_grid() {
    let mut g = make_grid(4, 2);
    g.print('X');
    assert_eq!(g.visible_row(0)[0].ch, 'X');
}

#[test]
fn visible_row_at_offset_one_shows_scrollback() {
    let mut g = make_grid(4, 2);
    g.print('A'); g.execute(0x0A);
    g.print('B'); g.execute(0x0A); // 'A' enters scrollback
    g.scroll_up_by(1);
    assert_eq!(g.visible_row(0)[0].ch, 'A');
}

#[test]
fn scroll_down_by_returns_to_live_view() {
    let mut g = make_grid(4, 2);
    g.print('A'); g.execute(0x0A);
    g.print('B'); g.execute(0x0A);
    g.scroll_up_by(1);
    g.scroll_down_by(1);
    assert_eq!(g.scroll_offset(), 0);
}

#[test]
fn scroll_up_by_clamped_to_scrollback_len() {
    let mut g = make_grid(4, 2);
    g.print('A'); g.execute(0x0A);
    g.print('B'); g.execute(0x0A);
    g.scroll_up_by(9999);
    assert_eq!(g.scroll_offset(), g.scrollback_len());
}

#[test]
fn scrollback_capped_at_10000_lines() {
    let mut g = make_grid(4, 2);
    for _ in 0..10_100 {
        g.print('X'); g.execute(0x0A);
    }
    assert!(g.scrollback_len() <= 10_000);
}
