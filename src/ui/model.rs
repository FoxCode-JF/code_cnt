use color_eyre::Result;
use ratatui::{
    Frame,
    crossterm::event::{self, Event, KeyCode},
    prelude::*,
    style::Stylize,
    widgets::{Block, Borders, Paragraph},
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
        let base_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(20), Constraint::Percentage(80)])
            .split(frame.area());

        let bar_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(60),
            ])
            .split(base_layout[1]);

        frame.render_widget(
            Paragraph::new("Count Lines like a pro!")
                .centered()
                .cyan()
                .block(Block::new().borders(Borders::BOTTOM)),
            base_layout[0],
        );
        frame.render_widget(
            Paragraph::new("Files Count")
                .centered()
                .cyan()
                .block(Block::new().borders(Borders::RIGHT)),
            bar_layout[0],
        );
        frame.render_widget(
            Paragraph::new("LoC 1234")
                .centered()
                .cyan()
                .block(Block::new().borders(Borders::LEFT | Borders::RIGHT)),
            bar_layout[1],
        );
        frame.render_widget(
            Paragraph::new("*************")
                .centered()
                .cyan()
                .block(Block::new().borders(Borders::LEFT)),
            bar_layout[2],
        );
    }
}
