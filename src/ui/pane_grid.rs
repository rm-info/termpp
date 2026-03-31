use std::sync::{Arc, Mutex};

use iced::mouse;
use iced::widget::canvas::{self, Frame, Path};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme};

use crate::terminal::grid::{GridPerformer, DEFAULT_BG};

/// Inset from the pane edge to where text begins (pixels).
pub const TERM_PADDING: f32 = 8.0;

/// A canvas program that renders a terminal grid.
struct TerminalProgram {
    grid: Arc<Mutex<GridPerformer>>,
    font_size: f32,
    font_name: &'static str,
    cursor_on: bool,
}

impl<Message> canvas::Program<Message, Theme, Renderer> for TerminalProgram {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry<Renderer>> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Fill background with DEFAULT_BG so gaps between glyphs match cell backgrounds
        let bg_rect = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&bg_rect, Color::from_rgb8(DEFAULT_BG.0, DEFAULT_BG.1, DEFAULT_BG.2));

        // Render grid cells
        if let Ok(grid) = self.grid.lock() {
            let cols = grid.cols();
            let rows = grid.rows();

            // Cell dimensions derived from font_size so glyphs never overlap.
            // These match the ratios used at emulator startup in app.rs.
            let cell_w = self.font_size * 0.6;
            let cell_h = self.font_size * 1.2;

            for row in 0..rows {
                let row_cells = grid.visible_row(row);
                for col in 0..cols {
                    if col >= row_cells.len() { break; }
                    let cell = &row_cells[col];
                    let x = TERM_PADDING + col as f32 * cell_w;
                    let y = TERM_PADDING + row as f32 * cell_h;

                    let bg = &cell.bg;
                    if (bg.0, bg.1, bg.2) != (DEFAULT_BG.0, DEFAULT_BG.1, DEFAULT_BG.2) {
                        let rect = Path::rectangle(Point::new(x, y), Size::new(cell_w, cell_h));
                        frame.fill(&rect, Color::from_rgb8(bg.0, bg.1, bg.2));
                    }

                    if cell.ch == ' ' || cell.ch == '\0' { continue; }

                    let fg = &cell.fg;
                    frame.fill_text(canvas::Text {
                        content: cell.ch.to_string(),
                        position: Point::new(x, y),
                        color: Color::from_rgb8(fg.0, fg.1, fg.2),
                        size: iced::Pixels(self.font_size),
                        font: iced::Font {
                            family: iced::font::Family::Name(self.font_name),
                            ..iced::Font::MONOSPACE
                        },
                        ..canvas::Text::default()
                    });
                }
            }

            // Cursor: only when at live view (scroll_offset == 0)
            if self.cursor_on && grid.scroll_offset() == 0 {
                let cx = TERM_PADDING + grid.cursor_col as f32 * cell_w;
                let cy = TERM_PADDING + grid.cursor_row as f32 * cell_h;
                let bar = Path::rectangle(
                    Point::new(cx, cy + cell_h - 3.0),
                    Size::new(cell_w, 3.0),
                );
                frame.fill(&bar, iced::Color { r: 0.85, g: 0.90, b: 1.0, a: 1.0 });
            }

            // Scrollback indicator
            if grid.scroll_offset() > 0 {
                let label = format!("↑ {} lignes", grid.scroll_offset());
                frame.fill_text(canvas::Text {
                    content: label,
                    position: Point::new(bounds.width - 90.0, TERM_PADDING),
                    color: Color { r: 0.39, g: 0.44, b: 0.53, a: 1.0 },
                    size: iced::Pixels(11.0),
                    ..canvas::Text::default()
                });
            }
        }

        vec![frame.into_geometry()]
    }
}

/// Widget that wraps a terminal grid and renders it via iced Canvas.
pub struct TerminalPane {
    grid: Arc<Mutex<GridPerformer>>,
    font_size: f32,
    font_name: &'static str,
    cursor_on: bool,
}

impl TerminalPane {
    pub fn new(
        grid: Arc<Mutex<GridPerformer>>,
        font_size: f32,
        font_name: &'static str,
        cursor_on: bool,
    ) -> Self {
        Self { grid, font_size, font_name, cursor_on }
    }

    pub fn view<Message: 'static>(&self) -> Element<'static, Message> {
        iced::widget::canvas(TerminalProgram {
            grid: Arc::clone(&self.grid),
            font_size: self.font_size,
            font_name: self.font_name,
            cursor_on: self.cursor_on,
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
    }
}
