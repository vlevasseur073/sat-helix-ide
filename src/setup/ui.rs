//! Simple ratatui selection UI for companion tools.

use crate::setup::catalog::{catalog_tools, Category, ToolId};
use crate::setup::detect::{should_offer_helix, tool_installed, HostInfo};
use crate::setup::plan::ToolSelection;
use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Debug)]
struct Row {
    kind: RowKind,
}

#[derive(Debug)]
enum RowKind {
    Header(Category),
    Tool(usize), // index into selections
}

#[derive(Debug)]
pub struct SetupApp {
    selections: Vec<ToolSelection>,
    rows: Vec<Row>,
    cursor: usize,
    dry_run: bool,
    confirmed: bool,
    quit: bool,
    host: HostInfo,
}

impl SetupApp {
    pub fn new(host: HostInfo, dry_run: bool) -> Self {
        let mut selections = Vec::new();
        for tool in catalog_tools() {
            if *tool == ToolId::Helix && !should_offer_helix() {
                continue;
            }
            let already = tool_installed(*tool);
            let methods = host.available_methods(*tool);
            let method = if methods.contains(&tool.default_method()) || methods.is_empty() {
                tool.default_method()
            } else {
                methods[0]
            };
            let selected = !already && tool.recommended();
            selections.push(ToolSelection {
                tool: *tool,
                method,
                selected,
                already_installed: already,
            });
        }

        let mut app = Self {
            selections,
            rows: Vec::new(),
            cursor: 0,
            dry_run,
            confirmed: false,
            quit: false,
            host,
        };
        app.rebuild_rows();
        // Focus first tool row
        if let Some(idx) = app
            .rows
            .iter()
            .position(|r| matches!(r.kind, RowKind::Tool(_)))
        {
            app.cursor = idx;
        }
        app
    }

    fn rebuild_rows(&mut self) {
        self.rows.clear();
        for category in Category::all() {
            let tools_in_cat: Vec<usize> = self
                .selections
                .iter()
                .enumerate()
                .filter(|(_, s)| s.tool.category() == *category)
                .map(|(i, _)| i)
                .collect();
            if tools_in_cat.is_empty() {
                continue;
            }
            self.rows.push(Row {
                kind: RowKind::Header(*category),
            });
            for idx in tools_in_cat {
                self.rows.push(Row {
                    kind: RowKind::Tool(idx),
                });
            }
        }
        if self.cursor >= self.rows.len() {
            self.cursor = self.rows.len().saturating_sub(1);
        }
    }

    pub fn run(mut self) -> Result<Option<Vec<ToolSelection>>> {
        let mut terminal = ratatui::init();
        let result = (|| -> Result<Option<Vec<ToolSelection>>> {
            loop {
                terminal
                    .draw(|frame| self.draw(frame))
                    .context("Failed to draw setup UI")?;
                self.handle_events()?;
                if self.quit {
                    return Ok(None);
                }
                if self.confirmed {
                    return Ok(Some(self.selections.clone()));
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

        let title = if self.dry_run {
            "sat-hx-ide setup (dry-run)"
        } else {
            "sat-hx-ide setup"
        };
        let header = Paragraph::new(Line::from(vec![
            Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  — select tools to install"),
        ]))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(header, chunks[0]);

        let mut lines: Vec<Line> = Vec::new();
        for (i, row) in self.rows.iter().enumerate() {
            let selected_row = i == self.cursor;
            match &row.kind {
                RowKind::Header(cat) => {
                    let style = if selected_row {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().add_modifier(Modifier::BOLD)
                    };
                    lines.push(Line::from(Span::styled(format!(" {}", cat.label()), style)));
                }
                RowKind::Tool(idx) => {
                    let sel = &self.selections[*idx];
                    let mark = if sel.selected { "[x]" } else { "[ ]" };
                    let status = if sel.already_installed {
                        "installed"
                    } else {
                        "missing"
                    };
                    let methods = self.host.available_methods(sel.tool);
                    let method = if methods.is_empty() {
                        format!("{} (unavailable)", sel.method)
                    } else {
                        sel.method.to_string()
                    };
                    let text = format!(
                        "  {mark} {:<10}  {status:<10}  method: {method}",
                        sel.tool.as_str()
                    );
                    let style = if selected_row {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(text, style)));
                }
            }
        }
        let list =
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Tools"));
        frame.render_widget(list, chunks[1]);

        let help = Paragraph::new(
            "[↑/↓] move  [Space] toggle  [m] cycle method  [a] toggle category  [Enter] confirm  [q] quit",
        )
        .block(Block::default().borders(Borders::ALL).title("Keys"));
        frame.render_widget(help, chunks[2]);
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
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Enter => self.confirmed = true,
            KeyCode::Up | KeyCode::Char('k') => self.move_cursor(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_cursor(1),
            KeyCode::Char(' ') => self.toggle_current(),
            KeyCode::Char('m') => self.cycle_method(),
            KeyCode::Char('a') => self.toggle_category(),
            _ => {}
        }
        Ok(())
    }

    fn move_cursor(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len() as isize;
        let mut next = self.cursor as isize + delta;
        while next >= 0 && next < len {
            if matches!(self.rows[next as usize].kind, RowKind::Tool(_)) {
                self.cursor = next as usize;
                return;
            }
            next += delta.signum();
        }
    }

    fn current_tool_index(&self) -> Option<usize> {
        match self.rows.get(self.cursor).map(|r| &r.kind) {
            Some(RowKind::Tool(idx)) => Some(*idx),
            _ => None,
        }
    }

    fn toggle_current(&mut self) {
        if let Some(idx) = self.current_tool_index() {
            let methods = self.host.available_methods(self.selections[idx].tool);
            if methods.is_empty() && !self.selections[idx].already_installed {
                return;
            }
            self.selections[idx].selected = !self.selections[idx].selected;
        }
    }

    fn cycle_method(&mut self) {
        let Some(idx) = self.current_tool_index() else {
            return;
        };
        let tool = self.selections[idx].tool;
        let methods = self.host.available_methods(tool);
        if methods.is_empty() {
            return;
        }
        let current = self.selections[idx].method;
        self.selections[idx].method = current.next(&methods);
    }

    fn toggle_category(&mut self) {
        let Some(idx) = self.current_tool_index() else {
            return;
        };
        let category = self.selections[idx].tool.category();
        let any_off = self
            .selections
            .iter()
            .filter(|s| s.tool.category() == category)
            .any(|s| !s.selected);
        for sel in &mut self.selections {
            if sel.tool.category() == category {
                let methods = self.host.available_methods(sel.tool);
                if any_off {
                    if !methods.is_empty() || sel.already_installed {
                        sel.selected = true;
                    }
                } else {
                    sel.selected = false;
                }
            }
        }
    }
}

pub fn run_ui(host: HostInfo, dry_run: bool) -> Result<Option<Vec<ToolSelection>>> {
    SetupApp::new(host, dry_run).run()
}
