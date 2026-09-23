//! ratatui selector for choosing a virtual environment in a floating Zellij pane.

use super::{
    normalize_selectable_venv, resolve_custom_venv_path, same_venv_path, SelectableVenv,
    VENV_SELECTOR_DONE, VENV_SELECTOR_OUTPUT, VENV_SELECTOR_SELECTION,
};
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectorRun {
    /// User chose an environment; handoff files were written.
    Selected,
    /// No environments were available; `NO_ENVIRONMENTS` marker written.
    NoEnvironments,
    /// User cancelled; no handoff output file.
    Cancelled,
}

#[derive(Debug)]
enum Mode {
    List,
    CustomPath {
        buffer: String,
        error: Option<String>,
    },
}

#[derive(Debug)]
struct VenvSelectorApp {
    environments: Vec<SelectableVenv>,
    active_path: Option<PathBuf>,
    runtime_dir: PathBuf,
    cursor: usize,
    mode: Mode,
    quit: bool,
    done: bool,
}

impl VenvSelectorApp {
    fn new(
        runtime_dir: PathBuf,
        environments: Vec<SelectableVenv>,
        active_path: Option<PathBuf>,
    ) -> Self {
        Self {
            environments,
            active_path,
            runtime_dir,
            cursor: 0,
            mode: Mode::List,
            quit: false,
            done: false,
        }
    }

    fn run(mut self) -> Result<SelectorRun> {
        let mut terminal = ratatui::init();
        let result = (|| -> Result<SelectorRun> {
            loop {
                terminal
                    .draw(|frame| self.draw(frame))
                    .context("Failed to draw venv selector")?;
                self.handle_events()?;
                if self.quit {
                    return Ok(SelectorRun::Cancelled);
                }
                if self.done {
                    return Ok(SelectorRun::Selected);
                }
            }
        })();
        ratatui::restore();
        result
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(area);

        let header = Paragraph::new(Line::from(vec![
            Span::styled(
                "Virtual environment",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  — select one to activate in the terminal"),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);

        match &self.mode {
            Mode::List => {
                let mut lines: Vec<Line> = Vec::new();
                lines.push(Line::from(Span::styled(
                    "  [x] active environment in this session",
                    Style::default().add_modifier(Modifier::DIM),
                )));
                lines.push(Line::from(""));
                for (i, env) in self.environments.iter().enumerate() {
                    let checkbox = if self
                        .active_path
                        .as_ref()
                        .is_some_and(|active| same_venv_path(active, &env.path))
                    {
                        "[x]"
                    } else {
                        "[ ]"
                    };
                    let path = env.path.display();
                    let text = format!(
                        "  {checkbox} {:<20}  {:<8}  {path}",
                        env.name, env.venv_type
                    );
                    let style = if i == self.cursor {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(text, style)));
                }
                let list = Paragraph::new(lines)
                    .block(Block::default().borders(Borders::ALL).title("Environments"));
                frame.render_widget(list, chunks[1]);

                let help = Paragraph::new(
                    "[↑/↓ j/k] move  [Space/Enter] select  [a] add custom path  [q/Esc] cancel",
                )
                .block(Block::default().borders(Borders::ALL).title("Keys"));
                frame.render_widget(help, chunks[2]);
            }
            Mode::CustomPath { buffer, error } => {
                let prompt = Paragraph::new(vec![
                    Line::from("Enter the path to a custom virtual environment:"),
                    Line::from(""),
                    Line::from(format!("> {buffer}")),
                ])
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Custom path"),
                );
                frame.render_widget(prompt, chunks[1]);

                let mut footer_lines = vec![Line::from(
                    "[Enter] confirm  [Esc] back  [Backspace] edit",
                )];
                if let Some(err) = error {
                    footer_lines.push(Line::from(Span::styled(
                        err.clone(),
                        Style::default().add_modifier(Modifier::ITALIC),
                    )));
                }
                let help = Paragraph::new(footer_lines)
                    .block(Block::default().borders(Borders::ALL).title("Keys"));
                frame.render_widget(help, chunks[2]);
            }
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        if !event::poll(std::time::Duration::from_millis(250))? {
            return Ok(());
        }
        let Event::Key(key) = event::read()? else {
            return Ok(());
        };
        if key.kind != KeyEventKind::Press {
            return Ok(());
        }

        match &mut self.mode {
            Mode::List => match key.code {
                KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
                KeyCode::Enter | KeyCode::Char(' ') => self.confirm_list_selection()?,
                KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
                KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    self.mode = Mode::CustomPath {
                        buffer: String::new(),
                        error: None,
                    };
                }
                _ => {}
            },
            Mode::CustomPath { buffer, error } => match key.code {
                KeyCode::Esc => {
                    let _ = error.take();
                    self.mode = Mode::List;
                }
                KeyCode::Enter => self.confirm_custom_path()?,
                KeyCode::Backspace => {
                    buffer.pop();
                    *error = None;
                }
                KeyCode::Char(ch) => {
                    buffer.push(ch);
                    *error = None;
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn move_cursor(&mut self, delta: isize) {
        if self.environments.is_empty() {
            return;
        }
        let len = self.environments.len() as isize;
        let next = (self.cursor as isize + delta).rem_euclid(len);
        self.cursor = next as usize;
    }

    fn confirm_list_selection(&mut self) -> Result<()> {
        let env = self
            .environments
            .get(self.cursor)
            .context("No environment selected")?
            .clone();
        write_list_handoff(&self.runtime_dir, &env)?;
        self.done = true;
        Ok(())
    }

    fn confirm_custom_path(&mut self) -> Result<()> {
        let Mode::CustomPath { buffer, error } = &mut self.mode else {
            return Ok(());
        };
        let trimmed = buffer.trim();
        if trimmed.is_empty() {
            self.quit = true;
            return Ok(());
        }
        match resolve_custom_venv_path(Path::new(trimmed)).and_then(normalize_selectable_venv) {
            Ok(resolved) => {
                write_list_handoff(&self.runtime_dir, &resolved)?;
                self.done = true;
            }
            Err(err) => {
                *error = Some(format!("{err:#}"));
            }
        }
        Ok(())
    }
}

fn write_list_handoff(runtime_dir: &Path, env: &SelectableVenv) -> Result<()> {
    let selection_file = runtime_dir.join(VENV_SELECTOR_SELECTION);
    let output_file = runtime_dir.join(VENV_SELECTOR_OUTPUT);
    fs::write(
        &selection_file,
        format!("{}\n", serde_json::to_string(env)?),
    )?;
    fs::write(&output_file, format!("{VENV_SELECTOR_DONE}\n"))?;
    Ok(())
}

/// Run the selector TUI, or write `NO_ENVIRONMENTS` when the list is empty.
pub fn run_selector_ui(
    runtime_dir: &Path,
    environments: Vec<SelectableVenv>,
    active_path: Option<&Path>,
) -> Result<SelectorRun> {
    if environments.is_empty() {
        fs::write(
            runtime_dir.join(VENV_SELECTOR_OUTPUT),
            "NO_ENVIRONMENTS\n",
        )?;
        return Ok(SelectorRun::NoEnvironments);
    }
    VenvSelectorApp::new(
        runtime_dir.to_path_buf(),
        environments,
        active_path.map(|p| p.to_path_buf()),
    )
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_environments_writes_no_environments_marker() {
        let dir = tempfile::tempdir().unwrap();
        let run = run_selector_ui(dir.path(), vec![], None).unwrap();
        assert_eq!(run, SelectorRun::NoEnvironments);
        let content =
            fs::read_to_string(dir.path().join(VENV_SELECTOR_OUTPUT)).unwrap();
        assert_eq!(content.trim(), "NO_ENVIRONMENTS");
    }
}
