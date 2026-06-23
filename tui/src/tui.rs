//! Terminal setup + async input loop. A dedicated tokio task owns a crossterm
//! `EventStream` and races it against tick/render timers, forwarding a unified
//! [`Event`] over an mpsc channel so the main loop never blocks on stdin.

use anyhow::Result;
use crossterm::event::{Event as CtEvent, EventStream, KeyEvent, KeyEventKind};
use futures::{FutureExt, StreamExt};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

const TICK_HZ: u64 = 4;
const RENDER_HZ: u64 = 30;

/// A unified UI event.
#[derive(Debug, Clone)]
pub enum Event {
    Tick,
    Render,
    Key(KeyEvent),
    Resize,
}

pub struct Tui {
    pub terminal: DefaultTerminal,
    rx: mpsc::UnboundedReceiver<Event>,
    task: JoinHandle<()>,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let terminal = ratatui::init();
        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(event_loop(tx));
        Ok(Self { terminal, rx, task })
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        self.task.abort();
        ratatui::restore();
    }
}

async fn event_loop(tx: mpsc::UnboundedSender<Event>) {
    let mut reader = EventStream::new();
    let mut tick = interval(Duration::from_millis(1000 / TICK_HZ));
    let mut render = interval(Duration::from_millis(1000 / RENDER_HZ));
    loop {
        let crossterm_event = reader.next().fuse();
        tokio::select! {
            _ = tick.tick() => {
                if tx.send(Event::Tick).is_err() { break; }
            }
            _ = render.tick() => {
                if tx.send(Event::Render).is_err() { break; }
            }
            maybe = crossterm_event => {
                match maybe {
                    Some(Ok(CtEvent::Key(key))) if key.kind == KeyEventKind::Press => {
                        if tx.send(Event::Key(key)).is_err() { break; }
                    }
                    Some(Ok(CtEvent::Resize(..))) => {
                        if tx.send(Event::Resize).is_err() { break; }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }
    }
}
