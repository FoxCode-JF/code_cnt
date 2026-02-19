use color_eyre::Result;
use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
    widgets::Paragraph,
};
use std::time::Duration;

use crate::ui::tui;

#[derive(Debug, Default)]
pub struct Model {
    running_state: RunningState,
}

#[derive(Debug, Default, PartialEq)]
enum RunningState {
    #[default]
    Running,
    Updating,
    Done,
}

#[derive(Debug, PartialEq)]
enum Message {
    Update,
    Quit,
}

impl Model {
    pub fn run(mut self) -> Result<()> {
        tui::install_panic_hook();
        let mut terminal = tui::init_terminal()?;
        let mut model = Model::default();

        while model.running_state != RunningState::Done {
            terminal.draw(|frame| self.view(frame))?;

            let current_msg = self.handle_event()?;

            if let Some(msg) = current_msg {
                model.running_state = self.update(msg)?
            } else {
                /* model.running_state does not change */
            };
        }

        tui::restore_terminal()?;
        Ok(())
    }
    fn update(&mut self, msg: Message) -> Result<RunningState> {
        match msg {
            Message::Update => Ok(RunningState::Updating),
            Message::Quit => Ok(RunningState::Done),
        }
    }

    fn handle_event(&mut self) -> Result<Option<Message>> {
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == event::KeyEventKind::Press {
                    return Ok(Self::handle_key(key));
                }
            }
        }

        Ok(None)
    }

    fn handle_key(key: event::KeyEvent) -> Option<Message> {
        match key.code {
            KeyCode::Char('u') => Some(Message::Update),
            KeyCode::Char('q') => Some(Message::Quit),
            _ => None,
        }
    }

    fn view(&mut self, frame: &mut Frame) {
        frame.render_widget(Paragraph::new("Count Lines like a pro!"), frame.area());
    }
}
