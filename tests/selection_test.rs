use termpp::terminal::grid::{GridPerformer, TermEvent};
use vte::Perform;

fn make_grid(cols: usize, rows: usize) -> GridPerformer {
    let (tx, _rx) = tokio::sync::mpsc::channel::<TermEvent>(32);
    GridPerformer::new(cols, rows, tx)
}

fn normalize_sel(
    sel: ((usize, usize), (usize, usize)),
) -> ((usize, usize), (usize, usize)) {
    let ((sc, sr), (ec, er)) = sel;
    if sr < er || (sr == er && sc <= ec) { sel } else { ((ec, er), (sc, sr)) }
}

fn extract(grid: &GridPerformer, sel: ((usize, usize), (usize, usize))) -> String {
    let ((sc, sr), (ec, er)) = normalize_sel(sel);
    let mut lines = Vec::new();
    for row in sr..=er {
        let start_col = if row == sr { sc } else { 0 };
        let end_col   = if row == er { ec } else { grid.cols().saturating_sub(1) };
        let row_cells = grid.visible_row(row);
        let text: String = row_cells
            .iter()
            .enumerate()
            .filter(|&(c, _)| c >= start_col && c <= end_col)
            .filter(|(_, cell)| cell.ch != '\0')
            .map(|(_, cell)| cell.ch)
            .collect::<String>()
            .trim_end()
            .to_string();
        lines.push(text);
    }
    lines.join("\n")
}

#[test]
fn extract_single_row() {
    let mut g = make_grid(10, 3);
    for ch in "hello".chars() { g.print(ch); }
    let text = extract(&g, ((0, 0), (4, 0)));
    assert_eq!(text, "hello");
}

#[test]
fn extract_normalises_reversed_selection() {
    let mut g = make_grid(10, 3);
    for ch in "hello".chars() { g.print(ch); }
    let text = extract(&g, ((4, 0), (0, 0)));
    assert_eq!(text, "hello");
}

#[test]
fn extract_multirow_joins_with_newline() {
    let mut g = make_grid(10, 3);
    for ch in "abcde".chars() { g.print(ch); }
    // CR+LF to move to start of next row (pure LF keeps col, hitting wrap on a 5-col grid)
    g.execute(0x0D);
    g.execute(0x0A);
    for ch in "fghij".chars() { g.print(ch); }
    let text = extract(&g, ((0, 0), (4, 1)));
    assert_eq!(text, "abcde\nfghij");
}

#[test]
fn extract_trims_trailing_spaces() {
    let mut g = make_grid(10, 3);
    g.print('A');
    let text = extract(&g, ((0, 0), (9, 0)));
    assert_eq!(text, "A");
}
