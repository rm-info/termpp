use tokio::sync::mpsc;

#[derive(Clone, Debug, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8);

pub const DEFAULT_FG: Color = Color(204, 204, 204);
pub const DEFAULT_BG: Color = Color(0, 0, 0);

const ANSI_COLORS: [Color; 8] = [
    Color(0,   0,   0),   Color(170, 0,   0),   Color(0,   170, 0),
    Color(170, 170, 0),   Color(0,   0,   170),  Color(170, 0,   170),
    Color(0,   170, 170), Color(170, 170, 170),
];

const ANSI_BRIGHT: [Color; 8] = [
    Color(85,  85,  85),  Color(255, 85,  85),  Color(85,  255, 85),
    Color(255, 255, 85),  Color(85,  85,  255),  Color(255, 85,  255),
    Color(85,  255, 255), Color(255, 255, 255),
];

#[derive(Clone, Debug)]
pub struct Cell {
    pub ch: char,
    pub fg: Color,
    pub bg: Color,
}

impl Cell {
    fn blank() -> Self {
        Self { ch: ' ', fg: DEFAULT_FG.clone(), bg: DEFAULT_BG.clone() }
    }
}

#[derive(Debug, Clone)]
pub enum TermEvent {
    Bell,
    OscNotify(String),
    Exited,
}

pub struct GridPerformer {
    cells: Vec<Vec<Cell>>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    cols: usize,
    rows: usize,
    current_fg: Color,
    current_bg: Color,
    event_tx: mpsc::Sender<TermEvent>,
}

impl GridPerformer {
    pub fn new(cols: usize, rows: usize, event_tx: mpsc::Sender<TermEvent>) -> Self {
        let blank_row = vec![Cell::blank(); cols];
        Self {
            cells: vec![blank_row; rows],
            cursor_col: 0, cursor_row: 0,
            cols, rows,
            current_fg: DEFAULT_FG.clone(),
            current_bg: DEFAULT_BG.clone(),
            event_tx,
        }
    }

    pub fn cell(&self, col: usize, row: usize) -> &Cell {
        &self.cells[row.min(self.rows - 1)][col.min(self.cols - 1)]
    }

    pub fn cols(&self) -> usize { self.cols }
    pub fn rows(&self) -> usize { self.rows }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.cells.resize_with(rows, || vec![Cell::blank(); cols]);
        for row in &mut self.cells { row.resize_with(cols, Cell::blank); }
        self.cols = cols;
        self.rows = rows;
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
    }

    fn write_char(&mut self, c: char) {
        if self.cursor_col >= self.cols { self.cursor_col = 0; self.advance_row(); }
        self.cells[self.cursor_row][self.cursor_col] =
            Cell { ch: c, fg: self.current_fg.clone(), bg: self.current_bg.clone() };
        self.cursor_col += 1;
    }

    fn advance_row(&mut self) {
        self.cursor_row += 1;
        if self.cursor_row >= self.rows { self.scroll_up(); self.cursor_row = self.rows - 1; }
    }

    fn scroll_up(&mut self) {
        self.cells.remove(0);
        self.cells.push(vec![Cell::blank(); self.cols]);
    }

    fn erase_from_cursor(&mut self) {
        for col in self.cursor_col..self.cols { self.cells[self.cursor_row][col] = Cell::blank(); }
        for row in (self.cursor_row + 1)..self.rows {
            for col in 0..self.cols { self.cells[row][col] = Cell::blank(); }
        }
    }

    fn clear_screen(&mut self) {
        for row in &mut self.cells { for cell in row.iter_mut() { *cell = Cell::blank(); } }
    }

    fn erase_line_from_cursor(&mut self) {
        for col in self.cursor_col..self.cols { self.cells[self.cursor_row][col] = Cell::blank(); }
    }

    fn erase_line(&mut self) {
        for col in 0..self.cols { self.cells[self.cursor_row][col] = Cell::blank(); }
    }

    fn apply_sgr(&mut self, params: &vte::Params) {
        for p in params.iter() {
            match p[0] as u8 {
                0        => { self.current_fg = DEFAULT_FG.clone(); self.current_bg = DEFAULT_BG.clone(); }
                30..=37  => { self.current_fg = ANSI_COLORS[(p[0] - 30) as usize].clone(); }
                39       => { self.current_fg = DEFAULT_FG.clone(); }
                40..=47  => { self.current_bg = ANSI_COLORS[(p[0] - 40) as usize].clone(); }
                49       => { self.current_bg = DEFAULT_BG.clone(); }
                90..=97  => { self.current_fg = ANSI_BRIGHT[(p[0] - 90) as usize].clone(); }
                100..=107 => { self.current_bg = ANSI_BRIGHT[(p[0] - 100) as usize].clone(); }
                _ => {}
            }
        }
    }

    fn fp(params: &vte::Params) -> usize {
        params.iter().next().and_then(|p| p.first().copied()).unwrap_or(0) as usize
    }
    fn sp(params: &vte::Params) -> usize {
        params.iter().nth(1).and_then(|p| p.first().copied()).unwrap_or(0) as usize
    }
}

impl vte::Perform for GridPerformer {
    fn print(&mut self, c: char) { self.write_char(c); }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => { let _ = self.event_tx.try_send(TermEvent::Bell); }
            0x08 => { if self.cursor_col > 0 { self.cursor_col -= 1; } }
            0x0A => { self.advance_row(); }
            0x0D => { self.cursor_col = 0; }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, c: char) {
        let p1 = Self::fp(params);
        let p2 = Self::sp(params);
        match c {
            'A' => { self.cursor_row = self.cursor_row.saturating_sub(p1.max(1)); }
            'B' => { self.cursor_row = (self.cursor_row + p1.max(1)).min(self.rows - 1); }
            'C' => { self.cursor_col = (self.cursor_col + p1.max(1)).min(self.cols - 1); }
            'D' => { self.cursor_col = self.cursor_col.saturating_sub(p1.max(1)); }
            'H' | 'f' => {
                self.cursor_row = p1.saturating_sub(1).min(self.rows - 1);
                self.cursor_col = p2.saturating_sub(1).min(self.cols - 1);
            }
            'J' => match p1 { 0 => self.erase_from_cursor(), 2 | 3 => self.clear_screen(), _ => {} },
            'K' => match p1 { 0 => self.erase_line_from_cursor(), 2 => self.erase_line(), _ => {} },
            'm' => { self.apply_sgr(params); }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() { return; }
        let cmd = std::str::from_utf8(params[0]).unwrap_or("");
        let notify = match cmd {
            "9"   => params.get(1).and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("").to_string(),
            "777" => params.get(3).and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("").to_string(),
            _ => return,
        };
        let _ = self.event_tx.try_send(TermEvent::OscNotify(notify));
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn make_grid(cols: usize, rows: usize) -> (GridPerformer, mpsc::Receiver<TermEvent>) {
        let (tx, rx) = mpsc::channel(32);
        (GridPerformer::new(cols, rows, tx), rx)
    }

    #[test]
    fn new_grid_is_blank() {
        let (g, _rx) = make_grid(80, 24);
        assert_eq!(g.cell(0, 0).ch, ' ');
        assert_eq!(g.cursor_col, 0);
        assert_eq!(g.cursor_row, 0);
    }

    #[test]
    fn print_writes_char_and_advances_cursor() {
        let (mut g, _rx) = make_grid(80, 24);
        vte::Perform::print(&mut g, 'A');
        assert_eq!(g.cell(0, 0).ch, 'A');
        assert_eq!(g.cursor_col, 1);
    }

    #[test]
    fn execute_newline_moves_cursor_down() {
        let (mut g, _rx) = make_grid(80, 24);
        vte::Perform::execute(&mut g, 0x0A);
        assert_eq!(g.cursor_row, 1);
    }

    #[test]
    fn execute_cr_resets_column() {
        let (mut g, _rx) = make_grid(80, 24);
        g.cursor_col = 10;
        vte::Perform::execute(&mut g, 0x0D);
        assert_eq!(g.cursor_col, 0);
    }

    #[test]
    fn execute_bel_sends_event() {
        let (mut g, mut rx) = make_grid(80, 24);
        vte::Perform::execute(&mut g, 0x07);
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn scroll_up_on_row_overflow() {
        let (mut g, _rx) = make_grid(80, 3);
        for _ in 0..3 {
            vte::Perform::print(&mut g, 'X');
            vte::Perform::execute(&mut g, 0x0A);
        }
        assert_eq!(g.cursor_row, 2);
    }
}
