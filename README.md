[![Stable](https://img.shields.io/badge/docs-stable-blue.svg)](https://vlevasseur073.github.io/sat-helix-ide/)

<div align="center">
<h1>
<picture>
<img src="book/src/assets/logo2.svg">
</picture>
</h1>
</div>

<!-- # sat-helix-ide --> 
## A workspace orchestrator for a full IDE.

`sat-hx-ide` is a small [Zellij](https://zellij.dev/) session composer for a [Helix](https://helix-editor.com/)-centered workflow.
enhanced with multiple additional tools among which:
[Yazi](https://yazi-rs.github.io/) +
[Lazygit](https://github.com/jesseduffield/lazygit)/[GitUI](https://github.com/gitui-org/gitui) +
[revdiff](https://github.com/umputun/revdiff) +
[glab-tui](https://github.com/rcieri/glab-tui) +
[Shiki](https://github.com/sazardev/shiki)
for instance

The features/tools currently implemented in `sat-helix-ide` are:
- a *File-Manager* (Default: Yazi)
- a *Git TUI* (Default: lazygit, tested with lazygit and gitui)
- a *review TUI* (Default: revdiff, tested with revdiff and `git diff`, possibly including git-delta)
- a *workflow TUI*; this should be understood as any kind of project management tool such as Github/Gitlab, jira, ... (Default: glab-tui)
- a *mind map TUI* (Default: [Shiki](https://github.com/sazardev/shiki)), docked beside Helix
- a *session status bar* (git branch/state, active venv, session name) on every tab
- *virtual environment* auto-detection, selector TUI, and terminal activation

`sat-hx-ide` starts a project session with:

- a `code` tab with Helix and a docked shell below it;
- an `ai` tab running the configured agent when it is installed;

Then, the different tools are accessible via zellij keybindings:
- `Ctrl-y` to open or focus Yazi in a full-screen floating pane;
- `Alt-y` to dock Yazi on the left of Helix, or float it again;
- `Alt-t` to hide the terminal behind fullscreen Helix, or show it again;
- `Alt-Shift-t` to zoom the terminal to full tab height, or dock it back;
- `Alt-m` to open or show the mind map docked beside Helix (or hide it again);
- `Alt-Shift-m` to zoom the mind map to full tab size, or dock it back;
- `Alt-g` to open Lazygit, GitUI, or another Git TUI in a floating pane;
- `Alt-r` to open the configured review tool (revdiff by default) in a floating pane.
- `Alt-w` to open the configured workflow tool (glab-tui by default) in a floating pane.
- `Alt-v` to activate or deactivate the selected Python virtual environment in the terminal.
- `Alt-Shift-v` to open the environment selector (ratatui TUI, including custom paths).


`sat-hx-ide` is deliberately **not** a dotfile manager: it does not rewrite your
Zellij, Helix, Yazi, or Git configs. Companion tools can be installed explicitly
with `sat-hx-ide setup` (opt-in).

> sat-hx-ide never writes your Zellij, Helix, Yazi, Lazygit, GitUI, or global
> Git configuration.

![`sat-hx-ide` demo](book/src/assets/demo.gif)

## How it works

Zellij has no configuration `include` directive. To add session-level
shortcuts without editing your config, sat-hx-ide:

1. reads your existing Zellij config;
2. parses it and checks for key collisions;
3. adds its bindings to a private runtime copy;
4. launches Zellij with `--config <runtime-copy>`.

The source remains byte-for-byte unchanged. Runtime files live under:

```text
$XDG_RUNTIME_DIR/sat-helix-ide/<session>/
├── config.kdl
└── layout.kdl
```

If `XDG_RUNTIME_DIR` is unavailable, the system temporary directory is used.

See [`docs/architecture.md`](docs/architecture.md) for init flow, helper commands,
and the pane cache.

## Requirements and installation

Required:

- Zellij
- Helix (`hx`)
- Yazi (the supported file-manager adapter)
- Lazygit, GitUI, or another configured Git TUI
- a current stable Rust toolchain when building from source

### Pre-built binary (Linux / macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/vlevasseur073/sat-helix-ide/main/install.sh | bash
sat-hx-ide doctor
```

Installs the latest GitHub release into `~/.local/bin` (override with `INSTALL_DIR`). Pin a version with `curl ... | VERSION=0.1.0 bash`.

Windows: download the `.zip` from [Releases](https://github.com/vlevasseur073/sat-helix-ide/releases).

### From source

```bash
cargo install --path .
# or, once published: cargo install sat-helix-ide
sat-hx-ide doctor
```

### Setup companion tools

After `sat-hx-ide` itself is installed, use the interactive setup TUI to install
recommended companions (Zellij, Helix, Yazi, Lazygit/GitUI, revdiff/delta,
gh/glab/glab-tui) via cargo, your package manager, or GitHub binaries:

```bash
sat-hx-ide setup
# Preview commands only:
sat-hx-ide setup --dry-run
```

Successful installs can update `~/.config/sat-helix-ide/config.toml` for the
selected tools in each category (comments in that file may be rewritten).

Shell aliases are invisible to Zellij. Configure an absolute executable path
when a command is not on `PATH`, such as Snap Helix:

```toml
[tools.editor]
command = "/snap/bin/hx"
```

## Usage

```bash
# Create or attach to a session named after the project directory
sat-hx-ide init .

# Skip the AI tab for this session
sat-hx-ide init . --no-ai

# Override the session name
sat-hx-ide init ~/src/project --session project-api

sat-hx-ide doctor
sat-hx-ide setup
sat-hx-ide config
sat-hx-ide version
```

An existing project-named session is attached by default. Set
`session.attach_existing = false` to fail instead.

## Workspace

The `code` tab runs Helix with your login shell docked beside it (15% by default,
below the editor). Set `terminal.dock_position = "right"` for a vertical split:

```text
┌─ code (dock down) ────────────────────────┐
│                  Helix                   │
├──────────────────────────────────────────┤
│               shell (15%)                │
└──────────────────────────────────────────┘

┌─ code (dock right) ───────┬───────────────┐
│                          │               │
│          Helix           │  shell (15%)  │
│                          │               │
└──────────────────────────┴───────────────┘
```

A second tab runs the configured AI agent whenever that command is on `PATH`:

```text
tabs: [ code: Helix ] [ ai: configured agent ] [ … new tabs … ]
```

Every tab includes a **sat-hx-ide status line** at the bottom (session name, git
branch/state, active venv). Optional Zellij input-mode bar: `[session] status_bar`.

The AI tab is best-effort. When `tools.ai.command` cannot be found, sat-hx-ide
prints a note and starts the session without that tab, so a machine without the
agent installed still works. Use `--no-ai` for a single session, or
`session.ai_by_default = false` to opt out permanently.

Yazi and the Git client are not permanent layout panes. They are opened on
demand by Zellij.

## Keybindings

Bindings are active in every Zellij mode except locked mode:

| Key | Action |
| --- | --- |
| `Ctrl-y` | Open Yazi as a full-screen floating pane, or focus the existing file-manager pane |
| `Alt-y` | Toggle the same file-manager pane between full-screen floating and docked left of Helix |
| `Alt-t` | Hide the terminal behind a fullscreen Helix, or show it again at the docked height |
| `Alt-Shift-t` | Zoom the terminal to full tab height, or dock it back to `terminal.dock_percent` |
| `Alt-g` | Open the configured Git client via `sat-hx-ide __git open` (floating pane) |
| `Alt-r` | Open the configured review tool via `sat-hx-ide __review open` (default: revdiff) |
| `Alt-v` | Toggle virtual environment activation in the terminal |
| `Alt-Shift-v` | Open the virtual environment selector TUI |
| `Ctrl-g` | Existing Zellij lock/unlock binding; sat-hx-ide intentionally leaves it alone |

The keys are configurable. sat-hx-ide refuses to launch if a selected key
already exists anywhere in the source Zellij keybindings; it never silently
replaces a user binding.

### File-manager states

```text
Ctrl-y
  missing  -> open floating
  existing -> focus it

Alt-y
  missing      -> open floating
  floating     -> dock left beside Helix
  docked left  -> float full-screen
```

When docked, Yazi takes `tools.file_manager.dock_percent` of the combined
Yazi/Helix width (28% by default):

```text
┌──────────────┬─────────────────────────────────┐
│              │                                 │
│     Yazi     │              Helix              │
│              │                                 │
└──────────────┴─────────────────────────────────┘
```

### Terminal states

The shell runs in a named `terminal` pane with no configured command — Zellij
starts your `$SHELL`. Toggles use Zellij fullscreen so the process keeps
running:

```text
Alt-t
  visible -> hide (Helix fullscreen, terminal stays alive underneath)
  hidden  -> show (restore split, focus terminal)

Alt-Shift-t
  docked -> zoom terminal to full tab height
  zoomed -> dock back to terminal.dock_percent
```

If you exit the shell, `Alt-t` or `Alt-Shift-t` spawns a fresh one.

Selecting a file in Yazi opens it in the existing named Helix pane. A floating
Yazi exits after selection. A docked Yazi reopens after handing the path to
Helix so the file-manager pane remains alongside the editor. Quitting Yazi
without a selection closes it in either state.

The integration uses targeted Zellij terminal input because Helix does not
provide a remote-control API.

## Configuration

sat-hx-ide reads:

```text
~/.config/sat-helix-ide/config.toml
```

Pass `--config` to use another path. A missing file is not created; bundled
defaults are used in memory. Copy the sample to customize:

```bash
mkdir -p ~/.config/sat-helix-ide
cp configs/config.toml ~/.config/sat-helix-ide/config.toml
```

Example (same as [`configs/config.toml`](configs/config.toml)):

```toml
[session]
# Attach when a session with the project-derived name already exists.
attach_existing = true
# Add the AI tab whenever the [tools.ai] agent below is on PATH. When it is
# not, the tab is skipped and the session still starts.
ai_by_default = true
# Optional explicit merge source. By default, ZELLIJ_CONFIG_FILE or the normal
# XDG path (~/.config/zellij/config.kdl) is used read-only.
# zellij_config = "/home/user/.config/zellij/config.kdl"
# Show Zellij's input-mode bar under the git/venv summary (summary is always on)
status_bar = true

[terminal]
# Dock a shell beside Helix in the code tab. It runs Zellij's default shell.
enabled = true
# Size of that shell as a percentage of the code tab (height when dock_position = "down", width when "right").
dock_percent = 15
# Terminal placement: "down" (default) or "right".
# dock_position = "down"

[mindmap]
# Include the mindmap pane in the code tab at session start.
# Off by default; Alt-m still opens it on demand when false.
enabled = false
# Size of that pane as a percentage of the code tab (height when dock_position = "down", width when "right").
dock_percent = 50
# dock placement: "down" or "right" (default).
# dock_position = "right"

[keybindings]
# Zellij key notation. Bindings are added to shared_except "locked".
file_manager = "Ctrl y"
file_manager_dock = "Alt y"
git = "Alt g"
review = "Alt r"
# Hide the terminal behind a fullscreen Helix, or bring it back.
terminal = "Alt t"
# Switch the terminal between its docked height and fullscreen.
terminal_zoom = "Alt Shift t"
mindmap = "Alt m"
mindmap_zoom = "Alt Shift m"
# Toggle virtual environment activation
venv = "Alt v"
# Open environment selector TUI
venv_select = "Alt Shift v"

[tools.zellij]
command = "zellij"
args = []

[tools.editor]
command = "hx"
args = []

[tools.file_manager]
adapter = "yazi"
command = "yazi"
args = []
float_width = "100%"
float_height = "100%"
dock_percent = 28

[tools.git]
command = "lazygit"
args = []

[tools.review]
command = "revdiff"
args = []

[tools.workflow]
command = "glab-tui"
args = []

# The agent for the AI tab. Ignored when the command is not on PATH.
[tools.ai]
command = "vibe"
args = []

[tools.mindmap]
command = "shiki"
args = []

# Virtual environment management
# Auto-detection is enabled by default when this section is not present
# or when auto_detection = true.
[venv]
# Enable venv management features (default: true)
enabled = true
# Path to a specific virtual environment to always include in selection
# Can be used alongside auto-detection from search paths
# path = ".venv"
# Type of environment for the configured path: "auto", "uv", "venv", "poetry", "conda"
# venv_type = "auto"
# Enable auto-detection of virtual environments (default: true)
auto_detection = true
# Additional directories to search for virtual environments
# Default searches the project directory. Add "~" to search home directory.
# search_paths = ["~", "/opt/venvs"]
```

The Zellij merge source is selected in this order:

1. `session.zellij_config`;
2. `ZELLIJ_CONFIG_FILE`;
3. `$XDG_CONFIG_HOME/zellij/config.kdl`;
4. `~/.config/zellij/config.kdl`.

If none exists, the private config contains only sat-hx-ide's additive
keybindings and Zellij supplies its normal defaults.

### Migration from the draft configuration

Old tool names remain readable:

- `[tools.helix] path` maps to `[tools.editor] command`;
- `[tools.yazi] path` maps to `[tools.file_manager] command`;
- `[tools.git] client` maps to `[tools.git] command`.

Draft-only fields (`layouts`, generated tool configs, themes, project
detection, delta settings) are ignored. Replace the file with the new sample
to remove ambiguity.

## Development

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
pre-commit run --all-files
```

GitHub Actions runs the formatter, Clippy, and tests on pushes and pull
requests.
