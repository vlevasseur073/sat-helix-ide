# Configuration

This chapter covers all configuration options available in `sat-helix-ide`. The tool is designed to be highly configurable while maintaining safety and non-intrusiveness.

## Configuration File

`sat-helix-ide` reads configuration from a TOML file located at:

```text
~/.config/sat-helix-ide/config.toml
```

### Configuration Locations

The configuration file is searched in the following order:

1. Path specified via `--config` flag
2. `~/.config/sat-helix-ide/config.toml` (default)
3. If not found, bundled defaults are used in memory

### Getting Started with Configuration

To create a custom configuration:

```bash
# Create the config directory
mkdir -p ~/.config/sat-helix-ide

# Copy the sample configuration
cp /path/to/sat-helix-ide/configs/config.toml ~/.config/sat-helix-ide/config.toml

# Edit the configuration
hx ~/.config/sat-helix-ide/config.toml
```

### Sample Configuration

Here's a complete sample configuration with all available options:

```toml
[session]
# Whether to attach to existing sessions by default
attach_existing = true

# Whether to include the AI tab by default
ai_by_default = true

# Show Zellij's input-mode status bar under the sat-hx-ide git/venv line.
# The sat-hx-ide summary line is always on; this only toggles Zellij's plugin.
status_bar = true

# Custom Zellij config path (optional)
# zellij_config = "/home/user/.config/zellij/config.kdl"

[terminal]
# Whether to include a terminal pane in the layout
enabled = true

# Terminal height as percentage when docked below editor
dock_percent = 15

# Terminal position relative to editor: "down" (horizontal) or "right" (vertical)
dock_position = "down"

[keybindings]
# File manager keybinding (default: Ctrl-y)
file_manager = "Ctrl y"

# File manager dock toggle keybinding (default: Alt-y)
file_manager_dock = "Alt y"

# Git TUI keybinding (default: Alt-g)
git = "Alt g"

# Review tool keybinding (default: Alt-r)
review = "Alt r"

# Workflow tool keybinding (default: Alt-w)
workflow = "Alt w"

# Terminal toggle keybinding (default: Alt-t)
terminal = "Alt t"

# Terminal zoom keybinding (default: Alt Shift t)
terminal_zoom = "Alt Shift t"

[tools.zellij]
# Path to Zellij executable
command = "zellij"
args = []

[tools.editor]
# Path to Helix executable
command = "hx"
args = []

[tools.file_manager]
# File manager adapter (currently only "yazi" is supported)
adapter = "yazi"

# Path to file manager executable
command = "yazi"
args = []

# Dimensions for floating file manager pane
float_width = "100%"
float_height = "100%"

# Width percentage when docked (28% by default)
dock_percent = 28

[tools.git]
# Git TUI client to use
command = "lazygit"  # or "gitui"
args = []

[tools.review]
# Code review tool to use
command = "revdiff"  # or "git diff", "git-delta"
args = []

[tools.workflow]
# Workflow/project management tool
command = "glab-tui"
args = []

[tools.ai]
# AI agent command (optional, skipped if not found)
command = "cursor-agent"  # or any other AI agent
args = []
```

## Session Settings

Settings related to Zellij session management.

### `session.attach_existing`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Whether to attach to an existing session with the same name instead of creating a new one

```toml
[session]
attach_existing = false  # Always create new session, fail if exists
```

### `session.ai_by_default`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Whether to include the AI tab by default

```toml
[session]
ai_by_default = false  # Don't create AI tab
```

### `session.zellij_config`

- **Type**: String (path)
- **Default**: Auto-detected from Zellij's configuration locations
- **Description**: Custom path to Zellij configuration file

```toml
[session]
zellij_config = "/path/to/custom/zellij/config.kdl"
```

### `session.status_bar`

- **Type**: Boolean
- **Default**: `false` in bundled defaults; sample config sets `true`
- **Description**: When `true`, append Zellij’s `zellij:status-bar` plugin below the sat-hx-ide git/venv summary on every tab. The sat-hx-ide summary itself is **always** enabled and is not controlled by this flag.

```toml
[session]
status_bar = false   # sat-hx-ide line only
status_bar = true    # sat-hx-ide line + Zellij input-mode bar
```

You can override for one session at init time: `sat-hx-ide init . --status-bar` (see `sat-hx-ide init --help`).

**Auto-detection order:**
1. `session.zellij_config` if specified
2. `ZELLIJ_CONFIG_FILE` environment variable
3. `$XDG_CONFIG_HOME/zellij/config.kdl`
4. `~/.config/zellij/config.kdl`

## Terminal Settings

Configuration for the integrated terminal pane.

### `terminal.enabled`

- **Type**: Boolean
- **Default**: `true`
- **Description**: Whether to include a terminal pane in the layout

```toml
[terminal]
enabled = false  # No terminal pane
```

### `terminal.dock_percent`

- **Type**: Integer
- **Default**: `15`
- **Description**: Terminal height as percentage of the split when docked
- **Range**: 1-99

```toml
[terminal]
dock_percent = 20  # Terminal takes 20% of the split
```

### `terminal.dock_position`

- **Type**: String
- **Default**: `"down"`
- **Valid values**: `"down"`, `"right"`
- **Description**: Position of terminal relative to editor

**Horizontal Split ("down"):**
```text
┌─────────────────────────┐
│      Editor             │
├─────────────────────────┤
│      Terminal           │
└─────────────────────────┘
```

**Vertical Split ("right"):**
```text
┌─────────────┬────────────┐
│   Editor     │  Terminal  │
│              │             │
└─────────────┴────────────┘
```

```toml
[terminal]
dock_position = "right"  # Vertical split
```

## Mind Map Configuration

The mind map is a docked companion pane (default tool: Shiki). It mirrors the
terminal's toggle/zoom model rather than the floating git/review tools.

### `mindmap.enabled`

- **Type**: Boolean
- **Default**: `false`
- **Description**: Include the mind map pane in the `code` tab at session start.
  When `false`, the pane is omitted until you press `Alt-m`. Unlike
  `terminal.enabled`, this does **not** block on-demand open.

```toml
[mindmap]
enabled = true  # Show Shiki beside Helix from session start
```

### `mindmap.dock_percent`

- **Type**: Integer
- **Default**: `50`
- **Description**: Width (`right`) or height (`down`) of the docked mind map as
  a percentage of the code tab
- **Range**: 1-99

```toml
[mindmap]
dock_percent = 40
```

### `mindmap.dock_position`

- **Type**: String
- **Default**: `"right"`
- **Valid values**: `"down"`, `"right"`
- **Description**: Position of the mind map relative to the editor

```toml
[mindmap]
dock_position = "right"
```

## Tool Configuration

Each tool has its own configuration section with `command` and `args` fields.

### `tools.zellij`

Configuration for the Zellij executable.

```toml
[tools.zellij]
command = "zellij"
args = ["--log-level", "warn"]  # Optional arguments
```

### `tools.editor`

Configuration for the editor (Helix by default).

```toml
[tools.editor]
command = "hx"
args = []

# For Snap-installed Helix
[tools.editor]
command = "/snap/bin/hx"
args = []
```

### `tools.file_manager`

Configuration for the file manager (Yazi by default).

```toml
[tools.file_manager]
adapter = "yazi"
command = "yazi"
args = []

# Custom file manager dimensions
float_width = "90%"
float_height = "90%"
dock_percent = 30
```

**Adapter**: Currently only `"yazi"` is supported as the file manager adapter.

### `tools.git`

Configuration for the Git TUI client.

```toml
[tools.git]
command = "lazygit"  # or "gitui"
args = []
```

### `tools.review`

Configuration for the code review tool.

```toml
[tools.review]
command = "revdiff"  # or "git diff", "git-delta"
args = []

# For git-delta with specific options
[tools.review]
command = "git"
args = ["diff", "--color=always", "--pager=delta"]
```

### `tools.workflow`

Configuration for the workflow/project management tool.

```toml
[tools.workflow]
command = "glab-tui"
args = []
```

### `tools.ai`

- **Type**: Object
- **Description**: Optional AI agent for the `ai` tab (skipped when missing)

```toml
[tools.ai]
command = "vibe"
args = []
```

### `tools.mindmap`

- **Type**: Object
- **Description**: Mind-mapping TUI docked beside Helix (default: Shiki)

```toml
[tools.mindmap]
command = "shiki"
args = []
```

## Keybinding Customization

All `sat-helix-ide` keybindings can be customized. The tool will **refuse to start** if your chosen keybindings conflict with existing Zellij keybindings.

### Available Keybindings

| Setting | Default | Description |
|---------|---------|-------------|
| `keybindings.file_manager` | `"Ctrl y"` | Open/focus file manager |
| `keybindings.file_manager_dock` | `"Alt y"` | Toggle file manager dock |
| `keybindings.git` | `"Alt g"` | Open Git TUI |
| `keybindings.review` | `"Alt r"` | Open review tool |
| `keybindings.workflow` | `"Alt w"` | Open workflow tool |
| `keybindings.terminal` | `"Alt t"` | Toggle terminal visibility |
| `keybindings.terminal_zoom` | `"Alt Shift t"` | Zoom terminal |
| `keybindings.mindmap` | `"Alt m"` | Toggle mind map dock |
| `keybindings.mindmap_zoom` | `"Alt Shift m"` | Zoom mind map |
| `keybindings.venv` | `"Alt v"` | Toggle venv activation in terminal |
| `keybindings.venv_select` | `"Alt Shift v"` | Open venv selector TUI |

### Customizing Keybindings

```toml
[keybindings]
file_manager = "Ctrl f"
file_manager_dock = "Alt f"
git = "Alt g"
review = "Alt d"
workflow = "Alt p"
terminal = "Alt t"
terminal_zoom = "Alt Shift t"
mindmap = "Alt n"
mindmap_zoom = "Alt Shift n"
venv = "Alt v"
venv_select = "Alt Shift v"
```

## Virtual Environment Settings

Configuration for Python (and related) environment detection and activation.

### `[venv]` section

| Setting | Default | Description |
|---------|---------|-------------|
| `enabled` | `true` | Enable venv keybindings and activation |
| `path` | unset | Optional fixed env path always included in the selector |
| `venv_type` | `"auto"` | Type for `path`: `auto`, `venv`, `uv`, `poetry`, `conda` |
| `auto_detection` | `true` | Scan project/search paths when toggling without a saved selection |
| `search_paths` | `[]` | Extra directories to scan (project dir is always searched) |

```toml
[venv]
enabled = true
# path = ".venv"
# venv_type = "auto"
auto_detection = true
# search_paths = ["~", "/opt/venvs"]
```

**Activation model:** Standard envs use `source <venv>/bin/activate` in the terminal pane. Entering a project directory that contains `.venv` in the selector resolves to that `.venv` directory. Poetry, Pipenv, and Conda use their usual shell commands when those project layouts are detected.

**Requirements:** `[venv] enabled = false` disables keybindings logic; the status bar still shows `venv none` unless you re-enable the feature.

### Keybinding Format

Keybindings use Zellij's keybinding syntax:

- Modifiers: `Ctrl`, `Alt`, `Shift`
- Keys: Letter keys (`a`, `b`, `c`, etc.), number keys (`1`, `2`, etc.), function keys (`F1`, `F2`, etc.)
- Multiple modifiers: `Ctrl Alt t`, `Alt Shift r`
- Space-separated: `"Ctrl y"`, `"Alt Shift t"`

### Keybinding Safety

`sat-helix-ide` performs the following safety checks:

1. **Parses your existing Zellij config** to detect all configured keybindings
2. **Checks for conflicts** with all `sat-helix-ide` keybindings
3. **Refuses to start** if any conflicts are found
4. **Never silently replaces** your existing keybindings

This ensures that your existing Zellij workflow is never accidentally broken.

### Example: Safe Keybinding Changes

```bash
# This will fail if Ctrl-f is already used in your Zellij config
sat-hx-ide init . --config ~/.config/sat-helix-ide/config.toml

# Error: Keybinding conflict: 'Ctrl f' is already bound in your Zellij config
```

To resolve conflicts:

1. Change the `sat-helix-ide` keybinding
2. Or remove the conflicting binding from your Zellij config
3. Or choose a different Zellij config via `session.zellij_config`

## Complete Configuration Example

Here's a comprehensive example showing various customizations:

```toml
[session]
attach_existing = true
ai_by_default = true

[terminal]
enabled = true
dock_percent = 20
dock_position = "right"

[mindmap]
enabled = false
dock_percent = 40
dock_position = "right"

[keybindings]
file_manager = "Ctrl f"
file_manager_dock = "Alt f"
git = "Alt g"
review = "Alt d"
workflow = "Alt p"
terminal = "Alt t"
terminal_zoom = "Alt Shift t"
mindmap = "Alt m"
mindmap_zoom = "Alt Shift m"

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
float_width = "95%"
float_height = "95%"
dock_percent = 25

[tools.git]
command = "lazygit"
args = []

[tools.review]
command = "git"
args = ["diff", "--color=always"]

[tools.workflow]
command = "glab-tui"
args = []

[tools.ai]
command = "vibe"
args = []

[tools.mindmap]
command = "shiki"
args = []
```

## Environment Variables

While most configuration is done via the TOML file, a few aspects can be influenced by environment variables:

### `XDG_RUNTIME_DIR`

- **Default**: `/run/user/<uid>/` on Linux
- **Purpose**: Directory for runtime files (session configs, layouts)
- **Fallback**: System temporary directory

```bash
# Override runtime directory
export XDG_RUNTIME_DIR=/tmp/my-runtime
sat-hx-ide init .
```

### `ZELLIJ_CONFIG_FILE`

- **Purpose**: Override Zellij configuration file path
- **Priority**: Higher than `session.zellij_config`

```bash
# Use a specific Zellij config
export ZELLIJ_CONFIG_FILE=/path/to/zellij/config.kdl
sat-hx-ide init .
```

### `RUST_LOG`

- **Purpose**: Control logging level for `sat-hx-ide`
- **Valid values**: `error`, `warn`, `info`, `debug`, `trace`

```bash
# Enable debug logging
RUST_LOG=debug sat-hx-ide init .

# Enable trace logging for maximum detail
RUST_LOG=trace sat-hx-ide init .
```

## Configuration Validation

`sat-helix-ide` validates your configuration on startup:

1. **File existence**: Checks that command executables exist and are accessible
2. **Keybinding conflicts**: Verifies no conflicts with Zellij bindings
3. **Value ranges**: Ensures numeric values are within valid ranges
4. **Required tools**: Validates that required tools (Zellij, Helix, Yazi) are available

### Running Configuration Check

```bash
# Check your configuration
sat-hx-ide config

# This will show:
# - Configuration file path
# - All resolved settings
# - Any warnings or errors
```

### Doctor Command

The `doctor` command performs a comprehensive system check:

```bash
sat-hx-ide doctor
```

This checks:
- All required tools are installed
- Configuration file can be read
- Keybindings don't conflict
- Runtime directory is writable

## Tips for Configuration

1. **Start with defaults**: The bundled defaults work well for most users
2. **Test changes**: After modifying configuration, test with a new session
3. **Check for conflicts**: Use `sat-hx-ide doctor` to verify your setup
4. **Backup your config**: Keep a backup of working configurations
5. **Document changes**: Add comments to your config file explaining custom settings

## Example: Development Configuration

For development on `sat-helix-ide` itself:

```toml
[session]
attach_existing = false  # Always create new session for development
ai_by_default = false    # No AI tab for development

[terminal]
enabled = true
dock_percent = 20
dock_position = "down"

[keybindings]
# Keep defaults for development

[tools.editor]
command = "hx"
args = []

[tools.file_manager]
adapter = "yazi"
command = "yazi"
args = []
dock_percent = 25

[tools.git]
command = "lazygit"
args = []

[tools.review]
command = "revdiff"
args = []

[tools.workflow]
command = "glab-tui"
args = []

# No AI for development
[tools.ai]
command = ""
```

## Next Steps

- [Architecture](./architecture.md) - Understand how `sat-helix-ide` works under the hood
- [Development](./development.md) - Learn about building and contributing
- [Troubleshooting](./troubleshooting.md) - Common issues and solutions
