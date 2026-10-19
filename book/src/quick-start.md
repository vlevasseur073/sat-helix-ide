# Quick Start

This chapter will get you up and running with `sat-helix-ide` in just a few minutes.

## Requirements

Before installing `sat-helix-ide`, ensure you have the following tools installed:

### Required Tools

| Tool | Purpose | Installation |
|------|---------|--------------|
| **Zellij** | Terminal workspace multiplexer | [Installation Guide](https://zellij.dev/documentation/installation.html) |
| **Helix** | Modal text editor | [Installation Guide](https://helix-editor.com/) |
| **Yazi** | Terminal file manager | [Installation Guide](https://yazi-rs.github.io/docs/installation) |

### Recommended Tools

| Tool | Purpose | Installation |
|------|---------|--------------|
| **Lazygit** | Git TUI client | [GitHub Releases](https://github.com/jesseduffield/lazygit/releases) |
| **GitUI** | Alternative Git TUI | [GitHub Releases](https://github.com/gitui-org/gitui/releases) |
| **revdiff** | Code review tool | [GitHub Releases](https://github.com/umputun/revdiff/releases) |
| **glab-tui** | GitHub/GitLab workflow | [GitHub Releases](https://github.com/rcieri/glab-tui/releases) |
| **Shiki** | Mind map / notes TUI | [GitHub](https://github.com/sazardev/shiki) / `cargo install shiki-cli` |
| **fzf** | Project picker fallback when no `PATH` is given | [GitHub](https://github.com/junegunn/fzf) |

### Build Requirements (for building from source)

- Rust toolchain (stable)
- Cargo

Verify your setup:

```bash
# Check Zellij
zellij --version

# Check Helix
hx --version

# Check Yazi
yazi --version
```

## Installation

### Option 1: Pre-built binary (Linux / macOS)

Downloads the latest GitHub release binary into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/vlevasseur073/sat-helix-ide/main/install.sh | bash
sat-hx-ide version
```

Optional environment variables:

| Variable | Default | Purpose |
|----------|---------|---------|
| `VERSION` | latest release | Pin a version (without leading `v`) |
| `INSTALL_DIR` | `~/.local/bin` | Install destination |
| `INSTALL_DESKTOP` | `1` on Linux | Set to `0` to skip the application-menu entry |

```bash
curl -fsSL https://raw.githubusercontent.com/vlevasseur073/sat-helix-ide/main/install.sh \
  | VERSION=0.1.0 INSTALL_DIR="$HOME/bin" bash
```

Windows: download the `.zip` from [Releases](https://github.com/vlevasseur073/sat-helix-ide/releases).

### Option 2: Install from source

```bash
git clone https://github.com/vlevasseur073/sat-helix-ide.git
cd sat-helix-ide
cargo install --path .
sat-hx-ide version
```

### Option 3: Install via Cargo (when published)

```bash
cargo install sat-helix-ide
```

### Setup companion tools

Install missing companions interactively (Zellij, Helix, Yazi, git/review/workflow
TUIs). Choose cargo, package manager, or pre-built binary per tool:

```bash
sat-hx-ide setup
sat-hx-ide setup --dry-run
```

### Post-Installation Check

Run the doctor command to verify your setup:

```bash
sat-hx-ide doctor
```

This will check that all required tools are installed and accessible.

## First Session

Let's start your first `sat-helix-ide` session:

```bash
# Pick a project interactively (Yazi or fzf), or launch from the desktop entry on Linux
sat-hx-ide

# Or, when you already know the project directory:
cd /path/to/your/project
sat-hx-ide .
```

With no `PATH`, sat-hx-ide opens the configured project picker (`[launcher]` in
`config.toml`; Yazi is preferred, with `fzf` as fallback when `picker = "auto"`).
Canceling the picker exits without starting a session.

Starting a session will:

1. Resolve your project directory (argument or picker)
2. Create a Zellij session named after your project
3. Write runtime configuration files to `$XDG_RUNTIME_DIR/sat-helix-ide/<session>/`
4. Launch Zellij with the configured layout

`init` and `start` remain optional aliases (`sat-hx-ide init .` is the same as `sat-hx-ide .`).

### What You'll See

After the session starts, you should see:

- A Zellij session with a **`code`** tab containing:
  - Helix editor in the main pane
  - A docked terminal below (15% height by default)
- An **`ai`** tab (if the configured AI agent is installed)
- A **status line** at the bottom of every tab: session name, git branch/state, and active venv

### Basic Navigation

| Action | Keybinding |
|--------|------------|
| Open file manager (Yazi) | `Ctrl-y` |
| Toggle file manager dock | `Alt-y` |
| Toggle terminal visibility | `Alt-t` |
| Zoom terminal | `Alt-Shift-t` |
| Toggle mind map dock | `Alt-m` |
| Zoom mind map | `Alt-Shift-m` |
| Open Git TUI | `Alt-g` |
| Open review tool | `Alt-r` |
| Open workflow tool | `Alt-w` |
| Toggle virtual env in terminal | `Alt-v` |
| Select virtual env (TUI) | `Alt-Shift-v` |

### Exiting the Session

- Use Zellij's default `Ctrl-q` to quit the session
- Or `Ctrl-d` to exit the current pane

### Reattaching to Session

If you exit and want to reattach:

```bash
# List existing sessions
zellij list-sessions

# Reattach to your project session
sat-hx-ide .
```

By default, `sat-hx-ide` attaches to an existing session with the same name. To create a new session instead (and fail if the name already exists), configure:

```toml
[session]
attach_existing = false
```

## Next Steps

- [User Manual](./user-manual.md) - Detailed guide to using all features
- [Configuration](./configuration.md) - Customize your setup
- [Keybindings](./user-manual.md#keybindings) - Learn all available shortcuts
