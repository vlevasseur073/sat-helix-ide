# sat-helix-ide
A workspace orchestrator for a full IDE based on Helix + Zellij + Yazi + lazygit/gitui + git-delta

[Helix](https://helix-editor.com/), [Zellij](https://zellij.dev/), [Yazi](https://yazi-rs.github.io/), and [Lazygit](https://github.com/jesseduffield/lazygit) or [GitUI](https://github.com/gitui-org/gitui) as one terminal IDE.


`sat-hx-ide` is a small Zellij session composer for a Helix-centered workflow.
It starts a project session with:

- a `code` tab with Helix and a docked shell below it;
- an `ai` tab running the configured agent when it is installed;
- `Ctrl-y` to open or focus Yazi in a full-screen floating pane;
- `Alt-y` to dock Yazi on the left of Helix, or float it again;
- `Alt-t` to hide the terminal behind fullscreen Helix, or show it again;
- `Alt-Shift-t` to zoom the terminal to full tab height, or dock it back;
- `Alt-g` to open Lazygit, GitUI, or another Git TUI in a floating pane;
- `Alt-r` to open the configured review tool (revdiff by default) in a floating pane.

It is deliberately **not** a dotfile manager or tool installer.

> sat-hx-ide never writes your Zellij, Helix, Yazi, Lazygit, GitUI, or global
> Git configuration.

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

```bash
cargo install --path .
sat-hx-ide doctor
```

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
tabs: [ code: Helix ] [ ai: configured agent ]
```

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

Example:

```toml
[session]
attach_existing = true
ai_by_default = true
# zellij_config = "/home/me/.config/zellij/config.kdl"

[terminal]
enabled = true
dock_percent = 15
# dock_position = "down"  # or "right" for a vertical editor/terminal split

[keybindings]
file_manager = "Ctrl y"
file_manager_dock = "Alt y"
git = "Alt g"
review = "Alt r"
terminal = "Alt t"
terminal_zoom = "Alt Shift t"

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
command = "lazygit" # or "gitui"
args = []

[tools.review]
command = "revdiff"
args = []

# Any agent command. Skipped silently when it is not installed.
[tools.ai]
command = "cursor-agent"
args = []
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
