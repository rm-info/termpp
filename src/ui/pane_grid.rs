use std::sync::{Arc, Mutex};

use iced::mouse;
use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme};

use crate::terminal::grid::GridPerformer;
use crate::ui::theme::Theme as AppTheme;

/// A canvas program that renders a terminal grid.
struct TerminalProgram {
    grid: Arc<Mutex<GridPerformer>>,
    is_waiting: bool,
    font_size: f32,
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

        // Fill background
        let bg_rect = Path::rectangle(Point::ORIGIN, bounds.size());
        frame.fill(&bg_rect, AppTheme::PANE_BG);

        // Render grid cells
        if let Ok(grid) = self.grid.lock() {
            let cols = grid.cols();
            let rows = grid.rows();

            // Compute character cell size based on available space
            let cell_w = if cols > 0 { bounds.width / cols as f32 } else { self.font_size };
            let cell_h = if rows > 0 { bounds.height / rows as f32 } else { self.font_size };

            for row in 0..rows {
                for col in 0..cols {
                    let cell = grid.cell(col, row);
                    if cell.ch == ' ' {
                        continue;
                    }

                    let x = col as f32 * cell_w;
                    let y = row as f32 * cell_h;

                    let fg = cell.fg.clone();
                    let color = Color::from_rgb8(fg.0, fg.1, fg.2);

                    let text = canvas::Text {
                        content: cell.ch.to_string(),
                        position: Point::new(x, y),
                        color,
                        size: iced::Pixels(cell_h.min(self.font_size * 1.5)),
                        ..canvas::Text::default()
                    };
                    frame.fill_text(text);
                }
            }
        }

        // Draw waiting ring outline
        if self.is_waiting {
            let inset = 2.0;
            let outline = Path::rectangle(
                Point::new(inset, inset),
                Size::new(bounds.width - inset * 2.0, bounds.height - inset * 2.0),
            );
            let stroke = Stroke::default()
                .with_color(AppTheme::RING_WAITING)
                .with_width(3.0);
            frame.stroke(&outline, stroke);
        }

        vec![frame.into_geometry()]
    }
}

/// Widget that wraps a terminal grid and renders it via iced Canvas.
pub struct TerminalPane {
    grid: Arc<Mutex<GridPerformer>>,
    is_waiting: bool,
    font_size: f32,
}

impl TerminalPane {
    pub fn new(grid: Arc<Mutex<GridPerformer>>, is_waiting: bool, font_size: f32) -> Self {
        Self { grid, is_waiting, font_size }
    }

    pub fn view<Message: 'static>(&self) -> Element<'static, Message> {
        iced::widget::canvas(TerminalProgram {
            grid: Arc::clone(&self.grid),
            is_waiting: self.is_waiting,
            font_size: self.font_size,
        })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
    }
}
