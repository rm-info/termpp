use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;
use crate::terminal::grid::{GridPerformer, TermEvent};
use crate::terminal::pty::Pty;

pub struct Emulator {
    pub grid:         Arc<Mutex<GridPerformer>>,
    pty:              Arc<Mutex<Pty>>,
    pub event_rx:     mpsc::Receiver<TermEvent>,
    /// Incremented each time the reader loop processes a chunk of PTY output.
    /// Used by the Tick handler to detect new output without holding any lock.
    pub output_count: Arc<AtomicU64>,
}

impl Emulator {
    /// Synchronous constructor — uses tokio::spawn internally for the PTY reader.
    /// Can be called from iced's application boot function without block_on.
    pub fn start(cols: u16, rows: u16) -> anyhow::Result<Self> {
        let (event_tx, event_rx) = mpsc::channel(64);
        let grid = Arc::new(Mutex::new(
            GridPerformer::new(cols as usize, rows as usize, event_tx.clone()),
        ));
        let pty = Arc::new(Mutex::new(Pty::spawn(cols, rows)?));
        let output_count = Arc::new(AtomicU64::new(0));

        let grid_c   = Arc::clone(&grid);
        let pty_c    = Arc::clone(&pty);
        let tx_c     = event_tx.clone();
        let count_c  = Arc::clone(&output_count);

        // Move the reader out of the mutex so the blocking read doesn't hold the pty
        // lock — the writer (write_input) needs the pty lock and must not be starved.
        // spawn_blocking keeps the blocking read off the async executor threads.
        let mut reader = {
            let mut p = pty_c.lock().unwrap_or_else(|e| e.into_inner());
            // Replace the reader field with a dummy that immediately returns EOF,
            // so the real reader can be moved into the background thread.
            // We use a cursor over an empty slice as a zero-cost placeholder.
            let real_reader = std::mem::replace(
                &mut p.reader,
                Box::new(std::io::Cursor::new(Vec::<u8>::new())),
            );
            real_reader
        };

        tokio::task::spawn_blocking(move || {
            let mut parser = vte::Parser::new();
            let mut buf    = [0u8; 4096];
            loop {
                let n = match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => { let _ = tx_c.try_send(TermEvent::Exited); break; }
                    Ok(n) => n,
                };
                count_c.fetch_add(1, Ordering::Relaxed);
                let mut g = grid_c.lock().unwrap_or_else(|e| e.into_inner());
                for &byte in &buf[..n] { parser.advance(&mut *g, byte); }
            }
        });

        Ok(Self { grid, pty, event_rx, output_count })
    }

    pub fn write_input(&self, data: &[u8]) -> std::io::Result<()> {
        self.pty.lock().unwrap_or_else(|e| e.into_inner()).write_input(data)
    }

    pub fn grid(&self) -> std::sync::MutexGuard<GridPerformer> {
        self.grid.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub async fn next_event(&mut self) -> Option<TermEvent> {
        self.event_rx.recv().await
    }
}
