# Introduction

## What is sat-helix-ide?

`sat-helix-ide` is a **workspace orchestrator** for a full IDE centered around the
[Helix](https://helix-editor.com/) editor. The project was **originally built on**
[Zellij](https://zellij.dev/), which remains the **default** session backend. A generic
[`SessionBackend`](https://github.com/vlevasseur073/sat-helix-ide/blob/main/src/backend/mod.rs)
layer now targets any supported **terminal multiplexer** (multi-tab / multi-pane shell).
[Kitty](https://sw.kovidgoyal.net/kitty/) is supported today via remote control (`kitten`).
You can launch sat-hx-ide from **any terminal emulator**; only the chosen backend must be installed.

At its core, `sat-helix-ide` creates and manages workspace sessions with pre-configured layouts,
keybindings, and tool integrations. Each shortcut spawns a short-lived `sat-hx-ide` helper that
drives the active backend and exits — there is **no background daemon**.

## Features

`sat-helix-ide` provides a unified interface to several best-in-class terminal tools:

| Category | Default Tool | Alternatives | Purpose |
|----------|-------------|--------------|---------|
| **Session backend** | Zellij | Kitty (`backend = "kitty"` or `"auto"`) | Hosts tabs/panes for the IDE layout |
| **Editor** | Helix (`hx`) | - | Modern modal text editor |
| **File Manager** | [Yazi](https://yazi-rs.github.io/) | - | Feature-rich terminal file manager |
| **Git TUI** | [Lazygit](https://github.com/jesseduffield/lazygit) | [GitUI](https://github.com/gitui-org/gitui) | Interactive Git client |
| **Review Tool** | [revdiff](https://github.com/umputun/revdiff) | `git diff`, git-delta | Code review interface |
| **Workflow Tool** | [glab-tui](https://github.com/rcieri/glab-tui) | - | GitHub/GitLab project management |
| **Mind Map** | [Shiki](https://github.com/sazardev/shiki) | - | Docked mind-mapping / notes TUI |

### Core Features

- **Pluggable session backend**: Zellij (default) or kitty; `auto` picks kitty when already inside a kitty window
- **Automatic Workspace Setup**: Create a session with Helix editor and optional AI assistant tab
- **Interactive Launcher**: Run `sat-hx-ide` with no path to pick a project via Yazi or `fzf`; Linux installs can add a desktop menu entry (`install-desktop`)
- **Docked Terminal**: Integrated shell pane that can be toggled or zoomed
- **File Manager Integration**: Open Yazi in floating or docked mode with smart focus management
- **Git Integration**: Launch Git TUI clients in floating panes
- **Review Tools**: Open diff tools for code review workflows
- **Workflow Tools**: Access project management TUIs (GitHub, GitLab, etc.)
- **Mind Map**: Dock [Shiki](https://github.com/sazardev/shiki) beside Helix for notes and mind maps (`Alt-m`)
- **Session Status Bar**: A one-line git / venv summary on **every tab** (including AI and user-created tabs), refreshed automatically
- **Virtual Environments**: Auto-detect Python envs in the project, pick one with a ratatui selector, toggle activation in the terminal (`Alt-v` / `Alt-Shift-v`)
- **Configuration Flexibility**: Customize every aspect without modifying your global configs
- **Non-Intrusive**: Never writes to your global Zellij, kitty, Helix, or tool configurations

## Project Philosophy

`sat-helix-ide` follows several key principles:

1. **Composition over Integration**: Rather than creating a monolithic IDE, it composes existing excellent tools
2. **Non-Intrusive**: It reads your configurations but never modifies them
3. **Short-Lived Processes**: Helper commands run briefly and exit, avoiding daemon complexity
4. **Configuration Safety**: Refuses to start if sat-hx-ide keybindings would conflict with your Zellij or kitty config
5. **Graceful Degradation**: Missing optional tools (like AI agents) are handled gracefully

> **Important**: `sat-helix-ide` is **not** a dotfile manager — it does not rewrite your Zellij, kitty, Helix, or Git configs. Use `sat-hx-ide setup` if you want an opt-in helper to install companion tools.

The project aims to provide a professional-grade development environment that feels native to terminal users while offering the convenience of an IDE.
