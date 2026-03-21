mod app;
mod config;
mod data;
mod event;
mod ui;

use std::io;

use clap::Parser;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, AppMode};
use config::{load_plan, prompt_plan_selection, save_plan};
use data::types::PlanType;
use data::usage::update_usage_interactive;
use event::{poll_event, AppEvent};

#[derive(Parser)]
#[command(name = "cctop", about = "htop-style Claude Code thread monitor")]
struct Cli {
    /// Plan type: pro, max5, max20
    #[arg(long)]
    plan: Option<String>,

    /// Update usage data interactively (paste /usage output)
    #[arg(long)]
    update_usage: bool,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    // Handle --update-usage subcommand
    if cli.update_usage {
        update_usage_interactive();
        return Ok(());
    }

    // Resolve plan: CLI arg → saved config → interactive prompt
    let plan = if let Some(plan_str) = &cli.plan {
        match PlanType::from_str(plan_str) {
            Some(p) => {
                save_plan(p);
                p
            }
            None => {
                eprintln!("Invalid plan '{}'. Use: pro, max5, max20", plan_str);
                std::process::exit(1);
            }
        }
    } else if let Some(p) = load_plan() {
        p
    } else {
        let p = prompt_plan_selection();
        save_plan(p);
        p
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let mut app = App::new(plan);
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| {
            ui::layout::render(frame, app);
        })?;

        if app.should_quit {
            return Ok(());
        }

        // Calculate remaining time until next refresh
        let elapsed = app.last_refresh.elapsed();
        let interval = app.refresh_interval();
        let timeout = interval.saturating_sub(elapsed);

        match poll_event(timeout) {
            Some(AppEvent::Key(key)) => {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Ctrl+C and Ctrl+D always quit
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('d'))
                {
                    app.should_quit = true;
                    continue;
                }

                match app.mode {
                    AppMode::Normal => match key.code {
                        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
                        KeyCode::Left | KeyCode::Char('h') => app.sort_prev(),
                        KeyCode::Right | KeyCode::Char('l') => app.sort_next(),
                        KeyCode::Char('q') => {
                            app.should_quit = true;
                        }
                        KeyCode::Char('d') | KeyCode::Delete => app.request_delete(),
                        KeyCode::F(2) | KeyCode::Char('s') => app.toggle_sort(),
                        KeyCode::F(5) | KeyCode::Char('r') => app.refresh_data(),
                        KeyCode::Char('u') => app.force_refresh_usage(),
                        _ => {}
                    },
                    AppMode::ConfirmDelete { .. } => match key.code {
                        KeyCode::Enter => app.confirm_delete(),
                        _ => app.cancel_delete(),
                    },
                }
            }
            Some(AppEvent::Tick) | None => {
                if app.needs_refresh() {
                    app.refresh_data();
                }
            }
        }
    }
}
