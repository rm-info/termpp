use std::collections::VecDeque;
use tokio::sync::mpsc;
use unicode_width::UnicodeWidthChar;

const SCROLLBACK_MAX: usize = 10_000;

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
    CwdChange(String),
    TitleChange(String),
    Exited,
}

fn ansi_256_color(n: u8) -> Color {
    match n {
        0..=7   => ANSI_COLORS[n as usize].clone(),
        8..=15  => ANSI_BRIGHT[(n - 8) as usize].clone(),
        16..=231 => {
            let idx = n - 16;
            let b = idx % 6;
            let g = (idx / 6) % 6;
            let r = idx / 36;
            let comp = |v: u8| if v == 0 { 0u8 } else { 55 + v * 40 };
            Color(comp(r), comp(g), comp(b))
        }
        232..=255 => {
            let v = 8 + (n - 232) * 10;
            Color(v, v, v)
        }
    }
}

pub struct GridPerformer {
    cells: VecDeque<Vec<Cell>>,
    pub cursor_col: usize,
    pub cursor_row: usize,
    saved_cursor: Option<(usize, usize)>,
    /// Backup of primary screen cells+cursor when alternate screen is active.
    alt_backup: Option<(VecDeque<Vec<Cell>>, usize, usize)>,
    cols: usize,
    rows: usize,
    current_fg: Color,
    current_bg: Color,
    reverse_video: bool,
    event_tx: mpsc::Sender<TermEvent>,
    scrollback:    VecDeque<Vec<Cell>>,
    scroll_offset: usize,
}

impl GridPerformer {
    pub fn new(cols: usize, rows: usize, event_tx: mpsc::Sender<TermEvent>) -> Self {
        Self {
            cells: (0..rows).map(|_| vec![Cell::blank(); cols]).collect(),
            cursor_col: 0, cursor_row: 0,
            saved_cursor: None,
            alt_backup: None,
            cols, rows,
            current_fg: DEFAULT_FG.clone(),
            current_bg: DEFAULT_BG.clone(),
            reverse_video: false,
            event_tx,
            scrollback:    VecDeque::new(),
            scroll_offset: 0,
        }
    }

    pub fn cell(&self, col: usize, row: usize) -> &Cell {
        &self.cells[row.min(self.rows - 1)][col.min(self.cols - 1)]
    }

    pub fn cols(&self) -> usize { self.cols }
    pub fn rows(&self) -> usize { self.rows }

    /// Returns the row to display for `visual_row` (0 = top of visible area),
    /// accounting for the current scroll_offset.
    pub fn visible_row(&self, visual_row: usize) -> &[Cell] {
        if self.scroll_offset == 0 {
            return &self.cells[visual_row.min(self.rows - 1)];
        }
        let abs_start = self.scrollback.len().saturating_sub(self.scroll_offset);
        let abs_idx   = abs_start + visual_row;
        if abs_idx < self.scrollback.len() {
            &self.scrollback[abs_idx]
        } else {
            let cell_idx = (abs_idx - self.scrollback.len()).min(self.rows - 1);
            &self.cells[cell_idx]
        }
    }

    /// Scroll back N lines into history (increases offset).
    pub fn scroll_up_by(&mut self, lines: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(self.scrollback.len());
    }

    /// Scroll forward N lines toward live view (decreases offset).
    pub fn scroll_down_by(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_offset(&self) -> usize { self.scroll_offset }
    pub fn scrollback_len(&self) -> usize { self.scrollback.len() }

    fn reset_scroll(&mut self) { self.scroll_offset = 0; }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        // Resize active screen
        while self.cells.len() < rows { self.cells.push_back(vec![Cell::blank(); cols]); }
        while self.cells.len() > rows { self.cells.pop_back(); }
        for row in &mut self.cells { row.resize_with(cols, Cell::blank); }
        // Also resize the alternate screen backup if present
        if let Some((alt, _, _)) = &mut self.alt_backup {
            while alt.len() < rows { alt.push_back(vec![Cell::blank(); cols]); }
            while alt.len() > rows { alt.pop_back(); }
            for row in alt.iter_mut() { row.resize_with(cols, Cell::blank); }
        }
        self.cols = cols;
        self.rows = rows;
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
    }

    fn write_char(&mut self, c: char) {
        if self.cursor_col >= self.cols { self.cursor_col = 0; self.advance_row(); }
        let w = UnicodeWidthChar::width(c).unwrap_or(1).max(1);
        let (fg, bg) = if self.reverse_video {
            (self.current_bg.clone(), self.current_fg.clone())
        } else {
            (self.current_fg.clone(), self.current_bg.clone())
        };
        self.cells[self.cursor_row][self.cursor_col] = Cell { ch: c, fg, bg };
        self.cursor_col += 1;
        if w == 2 && self.cursor_col < self.cols {
            let (fg2, bg2) = if self.reverse_video {
                (self.current_bg.clone(), self.current_fg.clone())
            } else {
                (self.current_fg.clone(), self.current_bg.clone())
            };
            self.cells[self.cursor_row][self.cursor_col] = Cell { ch: '\0', fg: fg2, bg: bg2 };
            self.cursor_col += 1;
        }
    }

    fn advance_row(&mut self) {
        self.cursor_row += 1;
        if self.cursor_row >= self.rows { self.scroll_up(); self.cursor_row = self.rows - 1; }
    }

    fn scroll_up(&mut self) {
        if let Some(displaced) = self.cells.pop_front() {
            if self.scrollback.len() >= SCROLLBACK_MAX {
                self.scrollback.pop_front();
            }
            self.scrollback.push_back(displaced);
        }
        self.cells.push_back(vec![Cell::blank(); self.cols]);
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

    fn erase_line_to_cursor(&mut self) {
        for col in 0..=self.cursor_col.min(self.cols - 1) {
            self.cells[self.cursor_row][col] = Cell::blank();
        }
    }

    fn erase_to_cursor(&mut self) {
        for row in 0..self.cursor_row {
            for col in 0..self.cols { self.cells[row][col] = Cell::blank(); }
        }
        self.erase_line_to_cursor();
    }

    fn apply_sgr(&mut self, params: &vte::Params) {
        let all: Vec<u16> = params.iter().flat_map(|p| p.iter().copied()).collect();
        let mut i = 0;
        while i < all.len() {
            match all[i] {
                0        => { self.current_fg = DEFAULT_FG.clone(); self.current_bg = DEFAULT_BG.clone(); self.reverse_video = false; }
                7        => { self.reverse_video = true; }
                27       => { self.reverse_video = false; }
                30..=37  => { self.current_fg = ANSI_COLORS[(all[i] - 30) as usize].clone(); }
                39       => { self.current_fg = DEFAULT_FG.clone(); }
                40..=47  => { self.current_bg = ANSI_COLORS[(all[i] - 40) as usize].clone(); }
                49       => { self.current_bg = DEFAULT_BG.clone(); }
                90..=97  => { self.current_fg = ANSI_BRIGHT[(all[i] - 90) as usize].clone(); }
                100..=107 => { self.current_bg = ANSI_BRIGHT[(all[i] - 100) as usize].clone(); }
                38 if i + 2 < all.len() && all[i + 1] == 5 => {
                    self.current_fg = ansi_256_color(all[i + 2] as u8);
                    i += 2;
                }
                38 if i + 4 < all.len() && all[i + 1] == 2 => {
                    self.current_fg = Color(all[i+2] as u8, all[i+3] as u8, all[i+4] as u8);
                    i += 4;
                }
                48 if i + 2 < all.len() && all[i + 1] == 5 => {
                    self.current_bg = ansi_256_color(all[i + 2] as u8);
                    i += 2;
                }
                48 if i + 4 < all.len() && all[i + 1] == 2 => {
                    self.current_bg = Color(all[i+2] as u8, all[i+3] as u8, all[i+4] as u8);
                    i += 4;
                }
                _ => {}
            }
            i += 1;
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

    fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], _ignore: bool, c: char) {
        // Private-mode sequences (intermediate = '?')
        if intermediates == [b'?'] {
            let p = Self::fp(params);
            match (p, c) {
                // Alternate screen: ?47h / ?1047h / ?1049h  — enter alternate screen
                (47 | 1047 | 1049, 'h') => {
                    let blank: VecDeque<Vec<Cell>> =
                        (0..self.rows).map(|_| vec![Cell::blank(); self.cols]).collect();
                    let primary = std::mem::replace(&mut self.cells, blank);
                    self.alt_backup = Some((primary, self.cursor_col, self.cursor_row));
                    self.cursor_col = 0;
                    self.cursor_row = 0;
                    self.reset_scroll();
                    self.current_fg = DEFAULT_FG.clone();
                    self.current_bg = DEFAULT_BG.clone();
                }
                // Alternate screen: ?47l / ?1047l / ?1049l  — exit alternate screen
                (47 | 1047 | 1049, 'l') => {
                    if let Some((primary, col, row)) = self.alt_backup.take() {
                        self.cells = primary;
                        self.cursor_col = col;
                        self.cursor_row = row;
                    }
                    self.reset_scroll();
                }
                _ => {} // cursor show/hide and other private modes ignored for now
            }
            return;
        }

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
            // Cursor movement
            'G' => { self.cursor_col = p1.saturating_sub(1).min(self.cols - 1); }
            'E' => { self.cursor_row = (self.cursor_row + p1.max(1)).min(self.rows - 1); self.cursor_col = 0; }
            'F' => { self.cursor_row = self.cursor_row.saturating_sub(p1.max(1)); self.cursor_col = 0; }
            // Erase
            'J' => match p1 { 0 => self.erase_from_cursor(), 1 => self.erase_to_cursor(), 2 | 3 => self.clear_screen(), _ => {} },
            'K' => match p1 { 0 => self.erase_line_from_cursor(), 1 => self.erase_line_to_cursor(), 2 => self.erase_line(), _ => {} },
            'X' => {
                // Erase p1 characters in place (no cursor movement)
                let n = p1.max(1).min(self.cols - self.cursor_col);
                for c in self.cursor_col..self.cursor_col + n { self.cells[self.cursor_row][c] = Cell::blank(); }
            }
            // Character insertion / deletion
            'P' => {
                // Delete p1 characters at cursor, shift rest of line left
                let n = p1.max(1).min(self.cols - self.cursor_col);
                let row = self.cursor_row;
                for c in self.cursor_col..self.cols - n { self.cells[row][c] = self.cells[row][c + n].clone(); }
                for c in self.cols - n..self.cols   { self.cells[row][c] = Cell::blank(); }
            }
            '@' => {
                // Insert p1 blank characters at cursor, shift rest of line right
                let n = p1.max(1).min(self.cols - self.cursor_col);
                let row = self.cursor_row;
                for c in (self.cursor_col..self.cols - n).rev() { self.cells[row][c + n] = self.cells[row][c].clone(); }
                for c in self.cursor_col..self.cursor_col + n     { self.cells[row][c] = Cell::blank(); }
            }
            // Line insertion / deletion
            'L' => {
                let n = p1.max(1);
                for _ in 0..n {
                    self.cells.insert(self.cursor_row, vec![Cell::blank(); self.cols]);
                    if self.cells.len() > self.rows { self.cells.pop_back(); }
                }
            }
            'M' => {
                let n = p1.max(1);
                for _ in 0..n {
                    if self.cursor_row < self.cells.len() {
                        self.cells.remove(self.cursor_row);
                        self.cells.push_back(vec![Cell::blank(); self.cols]);
                    }
                }
            }
            // Cursor save / restore (ANSI SCP/RCP)
            's' => { self.saved_cursor = Some((self.cursor_col, self.cursor_row)); }
            'u' => { if let Some((c, r)) = self.saved_cursor { self.cursor_col = c; self.cursor_row = r; } }
            'm' => { self.apply_sgr(params); }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() { return; }
        let cmd = std::str::from_utf8(params[0]).unwrap_or("");
        match cmd {
            "0" | "2" => {
                // OSC 0/2: set window/tab title (e.g. used by Claude Code, vim, etc.)
                let title = params.get(1)
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .unwrap_or("")
                    .to_string();
                if !title.is_empty() {
                    let _ = self.event_tx.try_send(TermEvent::TitleChange(title));
                }
            }
            "7" => {
                // OSC 7: shell reports cwd as file URI, e.g. file:///C:/Users/rmollon
                let raw = params.get(1)
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .unwrap_or("");
                let path = if let Some(p) = raw.strip_prefix("file://") {
                    // strip optional hostname before the path component
                    p.find('/').map(|i| &p[i..]).unwrap_or(p)
                } else {
                    raw
                };
                // On Windows, strip leading '/' before drive letter (e.g. /C:/ -> C:/)
                #[cfg(windows)]
                let path = path.trim_start_matches('/');
                let path = path.replace("%20", " ").replace("%3A", ":");
                if !path.is_empty() {
                    let _ = self.event_tx.try_send(TermEvent::CwdChange(path.to_string()));
                }
                return;
            }
            "9" => {
                let notify = params.get(1).and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("").to_string();
                let _ = self.event_tx.try_send(TermEvent::OscNotify(notify));
            }
            "777" => {
                // Format: 777;notify;title[;body] — use body if present, else title
                let notify = params.get(3)
                    .or_else(|| params.get(2))
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .unwrap_or("")
                    .to_string();
                if !notify.is_empty() {
                    let _ = self.event_tx.try_send(TermEvent::OscNotify(notify));
                }
            }
            _ => return,
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, byte: u8) {
        match byte {
            // DECSC / DECRC — save and restore cursor (DEC private sequences \x1b7 / \x1b8)
            b'7' => { self.saved_cursor = Some((self.cursor_col, self.cursor_row)); }
            b'8' => { if let Some((c, r)) = self.saved_cursor { self.cursor_col = c; self.cursor_row = r; } }
            // RI — reverse index: scroll down one line if at top, else cursor up
            b'M' => {
                if self.cursor_row == 0 {
                    self.cells.push_front(vec![Cell::blank(); self.cols]);
                    if self.cells.len() > self.rows { self.cells.pop_back(); }
                } else {
                    self.cursor_row -= 1;
                }
            }
            _ => {}
        }
    }
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
