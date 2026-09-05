# User Manual

This chapter provides a comprehensive guide to using `sat-helix-ide` effectively.

## Workspace Layout

`sat-helix-ide` creates a Zellij workspace with a carefully designed layout optimized for development workflows.

### Default Layout

```text
┌─────────────────────────────────────────────────────────────┐
│  Tabs: [ code* ] [ ai ]                                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                                                         │ │
│  │                    Helix Editor                       │ │
│  │                                                         │ │
│  │                                                         │ │
│  ├─────────────────────────────────────────────────────────┤ │
│  │                    Terminal (15%)                       │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Tab Structure

| Tab Name | Content | Purpose |
|----------|---------|---------|
| **`code`** | Helix + Terminal | Primary development workspace |
| **`ai`** | AI Agent | Optional AI assistant (when configured) |

### Workspace States

The workspace adapts dynamically based on your actions:

- **Normal State**: Helix editor with docked terminal visible
- **Terminal Hidden**: Helix takes full tab height, terminal hidden but still running
- **Terminal Zoomed**: Terminal takes full tab height, Helix hidden
- **File Manager Floating**: Yazi opens as full-screen floating pane
- **File Manager Docked**: Yazi docked to the left of Helix

## Keybindings

`sat-helix-ide` supports **two keybinding strategies** with different trade-offs:

### Keybinding Strategies Comparison

| Aspect | Zellij Keybindings | Helix Keybindings |
|--------|---------------------|-------------------|
| **Scope** | Works in any pane (file manager, terminal, etc.) | Only works in editor pane |
| **Mode Requirement** | Requires Zellij normal mode (not locked) | Works in any mode (locked or not) |
| **Availability** | Available everywhere in Zellij session | Only in Helix editor |
| **Flexibility** | Can trigger any `sat-hx-ide` action | Limited to Helix commands |
| **Integration** | Native Zellij integration | Uses Helix's keybinding system |
| **Best For** | Global workspace actions (file manager, terminal, tools) | Editor-specific actions |

#### Strategy Details

##### Zellij Keybindings (Primary Strategy)

All `sat-helix-ide` keybindings use **Zellij keybindings** as the primary strategy:

- Keybindings are active in **Zellij's normal mode** (not in locked mode)
- They work from **any pane** in the Zellij session
- They can trigger any `sat-hx-ide` helper command
- Press `Ctrl-g` to toggle Zellij's lock mode

**Example**: `Alt-y` to toggle file manager works whether you're in the editor, terminal, or any other pane.

##### Helix Keybindings (Alternative Strategy)

Alternatively, you can configure keybindings directly in **Helix** for editor-specific actions:

- Keybindings work **only in the Helix editor pane**
- They work **regardless of Zellij's lock mode**
- They use Helix's native keybinding system
- Limited to actions that can be expressed as Helix commands

**Example**: You could bind `Alt-y` in Helix to open Yazi, but it would only work from the editor pane:

```toml
[keys.normal.space.i]
f = ":sh sat-hx-ide __file-manager toggle-dock"
g = ":sh sat-hx-ide __git open"
t = ":sh sat-hx-ide __terminal toggle"
T = ":sh sat-hx-ide __terminal zoom"
r = ":sh sat-hx-ide __review open"
w = ":sh sat-hx-ide __workflow open"
```

This will work from your Helix editor in `sat-helix-ide`. It will also work if you run `hx` standalone inside any Zellij session.

### Primary Keybindings

| Key | Action | Description |
|-----|--------|-------------|
| `Ctrl-y` | File Manager Open | Open/focus Yazi in full-screen floating pane |
| `Alt-y` | File Manager Toggle | Toggle Yazi between floating and docked left of Helix |
| `Alt-t` | Terminal Toggle | Hide/show terminal via fullscreen |
| `Alt-Shift-t` | Terminal Zoom | Zoom terminal to full tab height or dock back |
| `Alt-g` | Git Open | Spawn Git TUI in floating pane |
| `Alt-r` | Review Open | Spawn review tool in floating pane |
| `Alt-w` | Workflow Open | Spawn workflow tool in floating pane |

### Keybinding Behavior Details

#### File Manager (`Ctrl-y` and `Alt-y`)

The file manager (Yazi by default) has two states:

```text
Ctrl-y behavior:
  No file manager pane exists -> Open Yazi in full-screen floating pane
  File manager pane exists    -> Focus the existing pane

Alt-y behavior:
  No file manager pane exists   -> Open Yazi in full-screen floating pane
  File manager is floating     -> Dock Yazi to the left of Helix
  File manager is docked left  -> Float Yazi full-screen
```

When docked, Yazi takes 28% of the combined width by default (configurable via `tools.file_manager.dock_percent`).

```text
┌──────────────┬─────────────────────────────────┐
│              │                                 │
│     Yazi     │              Helix              │
│    (28%)     │              (72%)              │
│              │                                 │
└──────────────┴─────────────────────────────────┘
```

**File Selection Behavior:**
- Floating Yazi: Exits after file selection, opens file in Helix
- Docked Yazi: Reopens after handing the path to Helix, stays visible
- Quitting Yazi without selection closes it in either state

#### Terminal (`Alt-t` and `Alt-Shift-t`)

The terminal runs in a named `terminal` pane with your `$SHELL`.

```text
Alt-t behavior:
  Terminal visible  -> Hide it (Helix fullscreen, terminal stays alive)
  Terminal hidden   -> Show it (restore split, focus terminal)

Alt-Shift-t behavior:
  Terminal docked  -> Zoom to full tab height
  Terminal zoomed  -> Dock back to terminal.dock_percent
```

**Terminal Respawn:**
If you exit the shell (e.g., `exit` or `Ctrl-d`), using `Alt-t` or `Alt-Shift-t` will spawn a fresh terminal with the same configuration.

#### Git TUI (`Alt-g`)

Opens the configured Git client (default: Lazygit) in a floating pane on the code tab. The Git TUI runs in its own process and can be closed independently.

#### Review Tool (`Alt-r`)

Opens the configured review tool (default: revdiff) in a floating pane. Useful for viewing diffs, commits, or code reviews.

#### Workflow Tool (`Alt-w`)

Opens the configured workflow tool (default: glab-tui) in a floating pane. This provides access to project management features like PR/MR management, issues, CI/CD, etc.

## File Manager Integration

### Yazi Integration

`sat-helix-ide` integrates tightly with Yazi for file management:

1. **Opening Files**: Selected files in Yazi are automatically opened in the Helix pane
2. **Smart Focus**: After file selection, focus returns appropriately
3. **State Management**: Yazi's pane state (floating/docked) is preserved across actions

### Configuration Options

```toml
[tools.file_manager]
adapter = "yazi"
command = "yazi"
args = []
float_width = "100%"
float_height = "100%"
dock_percent = 28
```

## Terminal Integration

The integrated terminal provides seamless access to your shell while maintaining the editor workflow.

### Terminal Configuration

```toml
[terminal]
enabled = true
dock_percent = 15
dock_position = "down"  # or "right" for vertical split
```

### Terminal Layout Options

**Horizontal Split (Default):**
```text
┌─ code (dock down) ────────────────────────┐
│                  Helix                   │
├──────────────────────────────────────────┤
│               shell (15%)                │
└──────────────────────────────────────────┘
```

**Vertical Split:**
```text
┌─ code (dock right) ───────┬───────────────┐
│                          │               │
│          Helix           │  shell (15%)  │
│                          │               │
└──────────────────────────┴───────────────┘
```

### Terminal States Visual Guide

```text
Normal State:
┌─────────────────────────────────────┐
│             Helix                     │
├─────────────────────────────────────┤
│            Terminal                   │
└─────────────────────────────────────┘

Alt-t (Hide Terminal):
┌─────────────────────────────────────┐
│             Helix                     │
│          (full height)                │
└─────────────────────────────────────┘

Alt-Shift-t (Zoom Terminal):
┌─────────────────────────────────────┐
│            Terminal                   │
│          (full height)                │
└─────────────────────────────────────┘
```

## Git Integration

`sat-helix-ide` provides seamless integration with Git TUI clients.

### Supported Git Clients

| Client | Description | Installation |
|--------|-------------|--------------|
| **Lazygit** | Feature-rich Git TUI | [GitHub](https://github.com/jesseduffield/lazygit) |
| **GitUI** | Alternative Git TUI | [GitHub](https://github.com/gitui-org/gitui) |

### Configuration

```toml
[tools.git]
command = "lazygit"  # or "gitui"
args = []
```

### Usage

1. Press `Alt-g` in any Zellij pane
2. A floating pane opens with the Git TUI
3. Perform Git operations as usual
4. Close the pane when done (the Git TUI continues running if needed)

### Git TUI Features via Integration

- View repository status
- Stage/unstage changes
- Commit changes
- View commit history
- Create and manage branches
- Resolve merge conflicts
- View diffs
- And more...

## Review Tools

Review tools help you examine code changes and perform code reviews.

### Supported Review Tools

| Tool | Description | Installation |
|------|-------------|--------------|
| **revdiff** | Smart diff viewer | [GitHub](https://github.com/umputun/revdiff) |
| **git diff** | Standard Git diff | Built-in |
| **git-delta** | Enhanced diff syntax highlighting | [GitHub](https://github.com/dandavison/delta) |

### Configuration

```toml
[tools.review]
command = "revdiff"
args = []
```

### Usage

1. Press `Alt-r` in any Zellij pane
2. A floating pane opens with the review tool
3. Navigate through changes, diffs, or commits
4. Close the pane when done

## Workflow Tools

Workflow tools provide access to project management and CI/CD features.

### Supported Workflow Tools

| Tool | Description | Installation |
|------|-------------|--------------|
| **glab-tui** | GitLab/GitHub TUI | [GitHub](https://github.com/rcieri/glab-tui) |

### Configuration

```toml
[tools.workflow]
command = "glab-tui"
args = []
```

### Workflow Tool Features

- View and manage pull requests / merge requests
- Browse and manage issues
- View CI/CD pipeline status
- Interact with project boards
- Manage repository settings
- And more...

## Using sat-helix-ide to Edit Itself

This section demonstrates how `sat-helix-ide` can be used to edit its own codebase, which is the basis for the screenshots in this documentation.

### Starting a Development Session

```bash
# Navigate to the sat-helix-ide repository
cd ~/Codes/sat-helix-ide

# Start a development session
sat-hx-ide init . --session sat-helix-ide-dev
```

### Typical Development Workflow

1. **Open the project in Helix**: The editor opens automatically with the repository
2. **Navigate with file manager**: `Ctrl-y` opens Yazi to browse the codebase
3. **Edit files**: Open files from Yazi or use Helix's file picker
4. **Run tests**: Use the terminal (`Alt-t` to toggle visibility) to run `cargo test`
5. **Check Git status**: `Alt-g` opens Lazygit to view changes
6. **Review changes**: `Alt-r` opens revdiff to see what you've modified
7. **Commit changes**: Use Lazygit to stage, commit, and push

### Multi-Pane Development

With the file manager docked (`Alt-y`):

```text
┌──────────────┬─────────────────────────────────┐
│              │                                 │
│   Yazi       │              Helix              │
│  (browsing   │           (editing)              │
│   files)     │                                 │
│              │                                 │
└──────────────┴─────────────────────────────────┘
       ▲
       │ Terminal docked below (hidden for now)
└─────────────────────────────────────────────────┘
```

This setup allows you to:
- Browse files in Yazi
- Edit the selected file in Helix
- Quickly toggle the terminal for builds and tests
- Access Git and review tools as needed

## Advanced Usage Patterns

### Working with Multiple Projects

```bash
# Project 1 in one session
cd ~/projects/project-a
sat-hx-ide init . --session project-a

# In another terminal:
cd ~/projects/project-b
sat-hx-ide init . --session project-b
```

Switch between sessions with Zellij:
```bash
zellij attach project-a
zellij attach project-b
```

### Custom Session Names

```bash
# Use a custom session name
sat-hx-ide init ~/src/myproject --session myproject-dev

# This creates a session named 'myproject-dev' instead of 'myproject'
```

### Disabling AI Tab

For projects where you don't need AI assistance:

```bash
# Temporarily for one session
sat-hx-ide init . --no-ai

# Permanently in configuration
# In ~/.config/sat-helix-ide/config.toml:
[session]
ai_by_default = false
```

## Tips and Tricks

1. **Quick file access**: Use `Ctrl-y` to quickly open Yazi, select a file, and return to editing
2. **Terminal shortcut**: `Alt-t` quickly hides/shows the terminal without changing focus
3. **Maximize editing space**: `Alt-t` to hide terminal, giving Helix full height
4. **Maximize terminal space**: `Alt-Shift-t` to zoom terminal for complex commands
5. **File manager dock**: `Alt-y` to keep Yazi visible while editing for easy file navigation
6. **Keybinding safety**: If a keybinding conflicts, `sat-hx-ide` will refuse to start

## Next Steps

- [Screenshots](./screenshots.md) - Visual examples of the workflow
- [Configuration](./configuration.md) - Customize your setup
- [Architecture](./architecture.md) - Understand how it works under the hood
