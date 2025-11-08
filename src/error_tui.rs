/// Error TUI: Interactive terminal UI for navigating and fixing errors
///
/// This module provides a beautiful terminal UI using ratatui for
/// navigating through compilation errors and applying fixes interactively.

use crate::error_mapper::{DiagnosticLevel, WindjammerDiagnostic};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io;

/// TUI application state
pub struct ErrorTui {
    /// List of diagnostics to display
    diagnostics: Vec<WindjammerDiagnostic>,
    /// Currently selected error index
    selected: usize,
    /// List state for the error list
    list_state: ListState,
    /// Whether to show help
    show_help: bool,
}

impl ErrorTui {
    /// Create a new TUI application
    pub fn new(diagnostics: Vec<WindjammerDiagnostic>) -> Self {
        let mut list_state = ListState::default();
        if !diagnostics.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            diagnostics,
            selected: 0,
            list_state,
            show_help: false,
        }
    }

    /// Run the TUI application
    pub fn run(&mut self) -> io::Result<Option<TuiAction>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Run the app
        let result = self.run_app(&mut terminal);

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    /// Main application loop
    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<Option<TuiAction>> {
        loop {
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                        KeyCode::Char('?') | KeyCode::F(1) => self.show_help = !self.show_help,
                        KeyCode::Down | KeyCode::Char('j') => self.next(),
                        KeyCode::Up | KeyCode::Char('k') => self.previous(),
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if !self.diagnostics.is_empty() {
                                return Ok(Some(TuiAction::ViewError(self.selected)));
                            }
                        }
                        KeyCode::Char('f') => {
                            if !self.diagnostics.is_empty() {
                                let diagnostic = &self.diagnostics[self.selected];
                                if diagnostic.is_fixable() {
                                    return Ok(Some(TuiAction::FixError(self.selected)));
                                }
                            }
                        }
                        KeyCode::Char('e') => {
                            if !self.diagnostics.is_empty() {
                                let diagnostic = &self.diagnostics[self.selected];
                                if let Some(code) = &diagnostic.code {
                                    return Ok(Some(TuiAction::ExplainError(code.clone())));
                                }
                            }
                        }
                        KeyCode::Char('a') => {
                            return Ok(Some(TuiAction::FixAll));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Move to next error
    fn next(&mut self) {
        if self.diagnostics.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.diagnostics.len();
        self.list_state.select(Some(self.selected));
    }

    /// Move to previous error
    fn previous(&mut self) {
        if self.diagnostics.is_empty() {
            return;
        }
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.diagnostics.len() - 1;
        }
        self.list_state.select(Some(self.selected));
    }

    /// Render the UI
    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),      // Title
                Constraint::Min(10),        // Main content
                Constraint::Length(3),      // Status bar
            ])
            .split(f.area());

        // Title
        self.render_title(f, chunks[0]);

        if self.show_help {
            // Show help screen
            self.render_help(f, chunks[1]);
        } else {
            // Split main area into error list and detail
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[1]);

            // Error list
            self.render_error_list(f, main_chunks[0]);

            // Error detail
            self.render_error_detail(f, main_chunks[1]);
        }

        // Status bar
        self.render_status_bar(f, chunks[2]);
    }

    /// Render title bar
    fn render_title(&self, f: &mut Frame, area: Rect) {
        let title = Paragraph::new("Windjammer Error Navigator")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, area);
    }

    /// Render error list
    fn render_error_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .diagnostics
            .iter()
            .enumerate()
            .map(|(i, diag)| {
                let (icon, color) = match diag.level {
                    DiagnosticLevel::Error => ("✗", Color::Red),
                    DiagnosticLevel::Warning => ("⚠", Color::Yellow),
                    DiagnosticLevel::Note => ("ℹ", Color::Blue),
                    DiagnosticLevel::Help => ("💡", Color::Green),
                };

                let code_str = if let Some(code) = &diag.code {
                    format!("[{}] ", code)
                } else {
                    String::new()
                };

                let fixable = if diag.is_fixable() { " [F]" } else { "" };

                let content = format!(
                    "{} {}{} - {}:{}{}",
                    icon,
                    code_str,
                    diag.message.chars().take(40).collect::<String>(),
                    diag.location.file.file_name().unwrap_or_default().to_string_lossy(),
                    diag.location.line,
                    fixable
                );

                ListItem::new(content).style(Style::default().fg(color))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(format!("Errors ({}/{})", self.selected + 1, self.diagnostics.len()))
                    .borders(Borders::ALL),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    /// Render error detail
    fn render_error_detail(&self, f: &mut Frame, area: Rect) {
        if self.diagnostics.is_empty() {
            let text = Paragraph::new("No errors to display!")
                .style(Style::default().fg(Color::Green))
                .block(Block::default().title("Detail").borders(Borders::ALL))
                .wrap(Wrap { trim: true });
            f.render_widget(text, area);
            return;
        }

        let diagnostic = &self.diagnostics[self.selected];

        let mut lines = vec![];

        // Error message
        let (level_str, level_color) = match diagnostic.level {
            DiagnosticLevel::Error => ("ERROR", Color::Red),
            DiagnosticLevel::Warning => ("WARNING", Color::Yellow),
            DiagnosticLevel::Note => ("NOTE", Color::Blue),
            DiagnosticLevel::Help => ("HELP", Color::Green),
        };

        lines.push(Line::from(vec![
            Span::styled(level_str, Style::default().fg(level_color).add_modifier(Modifier::BOLD)),
            Span::raw(": "),
            Span::raw(&diagnostic.message),
        ]));

        if let Some(code) = &diagnostic.code {
            lines.push(Line::from(vec![
                Span::styled("Code: ", Style::default().fg(Color::Cyan)),
                Span::raw(code),
            ]));
        }

        lines.push(Line::from(""));

        // Location
        lines.push(Line::from(vec![
            Span::styled("Location: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!(
                "{}:{}:{}",
                diagnostic.location.file.display(),
                diagnostic.location.line,
                diagnostic.location.column
            )),
        ]));

        lines.push(Line::from(""));

        // Help messages
        if !diagnostic.help.is_empty() {
            lines.push(Line::from(Span::styled(
                "Help:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )));
            for help in &diagnostic.help {
                lines.push(Line::from(format!("  • {}", help)));
            }
            lines.push(Line::from(""));
        }

        // Notes
        if !diagnostic.notes.is_empty() {
            lines.push(Line::from(Span::styled(
                "Notes:",
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            )));
            for note in &diagnostic.notes {
                lines.push(Line::from(format!("  • {}", note)));
            }
            lines.push(Line::from(""));
        }

        // Fixable status
        if diagnostic.is_fixable() {
            lines.push(Line::from(vec![
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::styled("This error can be fixed automatically!", Style::default().fg(Color::Green)),
            ]));
            lines.push(Line::from(Span::styled(
                "Press 'f' to apply fix",
                Style::default().fg(Color::Yellow),
            )));
        }

        let text = Paragraph::new(lines)
            .block(Block::default().title("Detail").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(text, area);
    }

    /// Render help screen
    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = vec![
            Line::from(Span::styled(
                "Keyboard Shortcuts",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("↑/k", Style::default().fg(Color::Yellow)),
                Span::raw("       - Previous error"),
            ]),
            Line::from(vec![
                Span::styled("↓/j", Style::default().fg(Color::Yellow)),
                Span::raw("       - Next error"),
            ]),
            Line::from(vec![
                Span::styled("Enter/Space", Style::default().fg(Color::Yellow)),
                Span::raw(" - View error details"),
            ]),
            Line::from(vec![
                Span::styled("f", Style::default().fg(Color::Yellow)),
                Span::raw("         - Fix current error (if fixable)"),
            ]),
            Line::from(vec![
                Span::styled("a", Style::default().fg(Color::Yellow)),
                Span::raw("         - Fix all fixable errors"),
            ]),
            Line::from(vec![
                Span::styled("e", Style::default().fg(Color::Yellow)),
                Span::raw("         - Explain error code"),
            ]),
            Line::from(vec![
                Span::styled("?/F1", Style::default().fg(Color::Yellow)),
                Span::raw("      - Toggle this help"),
            ]),
            Line::from(vec![
                Span::styled("q/Esc", Style::default().fg(Color::Yellow)),
                Span::raw("     - Quit"),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "Legend",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(vec![
                Span::styled("✗ ", Style::default().fg(Color::Red)),
                Span::raw("- Error"),
            ]),
            Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                Span::raw("- Warning"),
            ]),
            Line::from(vec![
                Span::styled("ℹ ", Style::default().fg(Color::Blue)),
                Span::raw("- Note"),
            ]),
            Line::from(vec![
                Span::styled("💡 ", Style::default().fg(Color::Green)),
                Span::raw("- Help"),
            ]),
            Line::from(vec![
                Span::styled("[F]", Style::default().fg(Color::Green)),
                Span::raw(" - Fixable"),
            ]),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().title("Help").borders(Borders::ALL))
            .wrap(Wrap { trim: true });

        f.render_widget(help, area);
    }

    /// Render status bar
    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let status = if self.show_help {
            "Press ? or F1 to close help"
        } else {
            "Press ? for help | ↑↓ to navigate | f to fix | a to fix all | e to explain | q to quit"
        };

        let status_bar = Paragraph::new(status)
            .style(Style::default().fg(Color::White).bg(Color::DarkGray))
            .block(Block::default());

        f.render_widget(status_bar, area);
    }
}

/// Action returned by the TUI
#[derive(Debug, Clone)]
pub enum TuiAction {
    /// View error details
    ViewError(usize),
    /// Fix a specific error
    FixError(usize),
    /// Fix all fixable errors
    FixAll,
    /// Explain an error code
    ExplainError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::Location;
    use std::path::PathBuf;

    fn create_test_diagnostic() -> WindjammerDiagnostic {
        WindjammerDiagnostic {
            message: "Test error".to_string(),
            level: DiagnosticLevel::Error,
            location: Location {
                file: PathBuf::from("test.wj"),
                line: 1,
                column: 1,
            },
            spans: vec![],
            code: Some("WJ0001".to_string()),
            help: vec!["This is a test".to_string()],
            notes: vec![],
        }
    }

    #[test]
    fn test_tui_creation() {
        let diagnostics = vec![create_test_diagnostic()];
        let tui = ErrorTui::new(diagnostics);
        assert_eq!(tui.selected, 0);
        assert_eq!(tui.diagnostics.len(), 1);
    }

    #[test]
    fn test_navigation() {
        let diagnostics = vec![
            create_test_diagnostic(),
            create_test_diagnostic(),
            create_test_diagnostic(),
        ];
        let mut tui = ErrorTui::new(diagnostics);

        assert_eq!(tui.selected, 0);
        tui.next();
        assert_eq!(tui.selected, 1);
        tui.next();
        assert_eq!(tui.selected, 2);
        tui.next();
        assert_eq!(tui.selected, 0); // Wraps around

        tui.previous();
        assert_eq!(tui.selected, 2); // Wraps around
        tui.previous();
        assert_eq!(tui.selected, 1);
    }
}

