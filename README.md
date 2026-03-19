# cctop (Claude Code Top)

An htop-style terminal UI monitor for Claude Code sessions.

## Overview

`cctop` is a lightweight, interactive Terminal User Interface (TUI) application written in Rust designed to help you track, analyze, and manage your Claude Code AI sessions. It automatically scans your local Claude Code data directories (`~/.claude/projects/`) and provides real-time insights into your token usage, session costs, and active sessions.

## Features

- **Interactive TUI**: Navigate and manage your Claude Code sessions using an intuitive, `htop`-like interface (built with `ratatui`).
- **Session Monitoring**: View all active and past sessions, sorted by recent activity.
- **Cost & Token Tracking**: Parses Claude Code's internal JSONL logs to accurately calculate token usage and estimate costs over a weekly rolling window.
- **Customizable Billing Plans**: Supports usage tracking against different API limits (`pro`, `max5`, `max20`).
- **Session Management**: Easily delete unwanted sessions (including their `subagents` and metadata directories) directly from the terminal.
- **Real-time Refresh**: Automatically polls for changes in active sessions.

## Tech Stack

- **Rust** (Edition 2021)
-  [Ratatui](https://github.com/ratatui-org/ratatui) & [Crossterm](https://github.com/crossterm-rs/crossterm) for terminal UI rendering and event handling.
-  [Clap](https://github.com/clap-rs/clap) for CLI argument parsing.
-  [Chrono](https://github.com/chronotope/chrono) for time calculations.
-  [Serde & Serde JSON](https://serde.rs/) for internal state and JSONL log parsing.

## Installation & Usage

Ensure you have Rust and Cargo installed.

```bash
# Build the project
cargo build --release

# Run cctop
cargo run --release
```

You can also launch it with a specific plan directly:
```bash
cargo run -- --plan pro
```

### Keybindings

- `Up` / `Down`: Navigate through sessions.
- `q`: Request deletion of the currently highlighted session.
- `y` / `Y`: Confirm deletion.
- `Ctrl+C`: Quit the application.

## Architecture

- `src/main.rs`: Entry point and CLI parsing.
- `src/app.rs`: Core application state and event loop logic.
- `src/ui/`: Ratatui layout, themes, and rendering logic.
- `src/data/`: Data models, JSONL parsing, pricing logic, and session discovery from `~/.claude/projects/`.

## License

MIT License
