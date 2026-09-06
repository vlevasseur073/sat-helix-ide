# Introduction

## What is sat-helix-ide?

`sat-helix-ide` is a **workspace orchestrator** that transforms [Zellij](https://zellij.dev/) into a full-fledged IDE centered around the [Helix](https://helix-editor.com/) editor. It composes a development environment by integrating multiple powerful terminal-based tools into a cohesive workflow.

At its core, `sat-helix-ide` is a small command-line tool that creates and manages Zellij sessions with pre-configured layouts, keybindings, and tool integrations. Each keybinding spawns a short-lived `sat-helix-ide` helper process that drives Zellij CLI actions and exits — there is **no background daemon**.

## Features

`sat-helix-ide` provides a unified interface to several best-in-class terminal tools:

| Category | Default Tool | Alternatives | Purpose |
|----------|-------------|--------------|---------|
| **Editor** | Helix (`hx`) | - | Modern modal text editor |
| **File Manager** | [Yazi](https://yazi-rs.github.io/) | - | Feature-rich terminal file manager |
| **Git TUI** | [Lazygit](https://github.com/jesseduffield/lazygit) | [GitUI](https://github.com/gitui-org/gitui) | Interactive Git client |
| **Review Tool** | [revdiff](https://github.com/umputun/revdiff) | `git diff`, git-delta | Code review interface |
| **Workflow Tool** | [glab-tui](https://github.com/rcieri/glab-tui) | - | GitHub/GitLab project management |

### Core Features

- **Automatic Workspace Setup**: Create a Zellij session with Helix editor and optional AI assistant tab
- **Docked Terminal**: Integrated shell pane that can be toggled or zoomed
- **File Manager Integration**: Open Yazi in floating or docked mode with smart focus management
- **Git Integration**: Launch Git TUI clients in floating panes
- **Review Tools**: Open diff tools for code review workflows
- **Workflow Tools**: Access project management TUIs (GitHub, GitLab, etc.)
- **Configuration Flexibility**: Customize every aspect without modifying your global configs
- **Non-Intrusive**: Never writes to your global Zellij, Helix, or tool configurations

## Project Philosophy

`sat-helix-ide` follows several key principles:

1. **Composition over Integration**: Rather than creating a monolithic IDE, it composes existing excellent tools
2. **Non-Intrusive**: It reads your configurations but never modifies them
3. **Short-Lived Processes**: Helper commands run briefly and exit, avoiding daemon complexity
4. **Configuration Safety**: Refuses to start if keybindings would conflict with your existing setup
5. **Graceful Degradation**: Missing optional tools (like AI agents) are handled gracefully

> **Important**: `sat-helix-ide` is **not** a dotfile manager — it does not rewrite your Zellij, Helix, or Git configs. Use `sat-hx-ide setup` if you want an opt-in helper to install companion tools.

The project aims to provide a professional-grade development environment that feels native to terminal users while offering the convenience of an IDE.
