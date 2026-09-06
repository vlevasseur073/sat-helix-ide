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
# Navigate to your project directory
cd /path/to/your/project

# Initialize a new session
sat-hx-ide init .
```

This command will:

1. Resolve your project directory
2. Create a Zellij session named after your project
3. Write runtime configuration files to `$XDG_RUNTIME_DIR/sat-helix-ide/<session>/`
4. Launch Zellij with the configured layout

### What You'll See

After running `sat-hx-ide init .`, you should see:

- A Zellij session with a **`code`** tab containing:
  - Helix editor in the main pane
  - A docked terminal below (15% height by default)
- An **`ai`** tab (if the configured AI agent is installed)

### Basic Navigation

| Action | Keybinding |
|--------|------------|
| Open file manager (Yazi) | `Ctrl-y` |
| Toggle file manager dock | `Alt-y` |
| Toggle terminal visibility | `Alt-t` |
| Zoom terminal | `Alt-Shift-t` |
| Open Git TUI | `Alt-g` |
| Open review tool | `Alt-r` |
| Open workflow tool | `Alt-w` |

### Exiting the Session

- Use Zellij's default `Ctrl-q` to quit the session
- Or `Ctrl-d` to exit the current pane

### Reattaching to Session

If you exit and want to reattach:

```bash
# List existing sessions
zellij list-sessions

# Reattach to your project session
sat-hx-ide init .
```

By default, `sat-hx-ide` will attach to an existing session with the same name. To disable this behavior:

```bash
sat-hx-ide init . --no-attach
```

Or configure it permanently in your `config.toml`:

```toml
[session]
attach_existing = false
```

## Next Steps

- [User Manual](./user-manual.md) - Detailed guide to using all features
- [Configuration](./configuration.md) - Customize your setup
- [Keybindings](./user-manual.md#keybindings) - Learn all available shortcuts
