# sat-helix-ide Architecture Design Document

## Table of Contents

1. [Overview](#overview)
2. [Architecture Components](#architecture-components)
3. [Action System Design](#action-system-design)
4. [Data Flow Diagrams](#data-flow-diagrams)
5. [Key Design Decisions](#key-design-decisions)
6. [Pros and Cons](#pros-and-cons)
7. [Component Relationships](#component-relationships)

---

## Overview

**sat-helix-ide** is a workspace orchestrator that composes a focused Zellij session centered around the Helix text editor. It provides a cohesive development environment with integrated tools (file manager, terminal, Git client, AI assistant) accessible through configurable keybindings.

### Core Purpose

- **Unified Workspace**: Create project-specific Zellij sessions with Helix as the primary editor
- **Tool Integration**: Seamlessly integrate external tools (file manager, Git, AI) into the workflow
- **Keybinding Abstraction**: Provide consistent keybindings that work across different tools and configurations
- **Configuration Management**: Generate runtime-specific Zellij configurations without modifying user's base config

---

## Architecture Components

### 1. Command Line Interface Layer (`src/main.rs`)

**Responsibility**: Entry point and command routing.

```
┌─────────────────────────────────────────┐
│            CLI Layer (main.rs)            │
├─────────────────────────────────────────┤
│  ✓ Parses command-line arguments (clap) │
│  ✓ Routes to appropriate handlers       │
│  ✓ Manages hidden internal commands     │
│  ✓ Handles verbose/error reporting       │
└─────────────────────────────────────────┘
```

**Components:**
- `Cli` struct with subcommands (`Init`, `Config`, `Doctor`, `Version`)
- Hidden commands (`__file-manager`, `__terminal`) for keybinding callbacks
- Command dispatch logic that maps CLI to action handlers

### 2. Workspace Manager (`src/workspace/manager.rs`)

**Responsibility**: Session lifecycle management.

```
┌─────────────────────────────────────────┐
│        Workspace Manager                   │
├─────────────────────────────────────────┤
│  ✓ Project directory resolution          │
│  ✓ Session name generation                │
│  ✓ Zellij session creation/attachment    │
│  ✓ Runtime directory management           │
│  ✓ Layout and config generation           │
│  ✓ Tool path resolution                   │
│  ✓ AI tab availability checking          │
└─────────────────────────────────────────┘
```

### 3. Zellij Integration Layer (`src/zellij/`)

**Responsibility**: Zellij-specific operations and configuration generation.

```
┌─────────────────────────────────────────┐
│         Zellij Layer (zellij/)            │
├─────────────────────────────────────────┤
│  client.rs:                               │
│    ✓ Session status checking             │
│    ✓ Session creation/attachment          │
│    ✓ Session deletion                    │
│  layout.rs:                               │
│    ✓ KDL layout generation               │
│    ✓ Pane positioning and sizing         │
│    ✓ Tab configuration                   │
│  runtime_config.rs:                      │
│    ✓ Runtime config generation            │
│    ✓ Keybinding injection                │
│    ✓ Config merging with user's config   │
└─────────────────────────────────────────┘
```

### 4. Action Handlers (`src/actions.rs`)

**Responsibility**: Execute user-initiated actions during a session.

```
┌─────────────────────────────────────────┐
│          Action Handlers                   │
├─────────────────────────────────────────┤
│  FileManagerAction:                       │
│    ✓ Open - Focus or create file manager  │
│    ✓ ToggleDock - Toggle docked/floating  │
│    ✓ Run - Execute file manager in pane   │
│  TerminalAction:                          │
│    ✓ Toggle - Hide/show terminal         │
│    ✓ Zoom - Toggle terminal fullscreen    │
│  Shared Utilities:                        │
│    ✓ Pane discovery and management        │
│    ✓ Zellij command execution             │
│    ✓ Process spawning and monitoring      │
└─────────────────────────────────────────┘
```

### 5. Configuration Layer (`src/config/`)

**Responsibility**: Configuration loading, validation, and default management.

```
┌─────────────────────────────────────────┐
│          Configuration Layer                │
├─────────────────────────────────────────┤
│  mod.rs:                                  │
│    ✓ Config struct and defaults           │
│    ✓ Configuration file loading           │
│    ✓ Tilde expansion                     │
│    ✓ Session configuration               │
│    ✓ Terminal configuration              │
│    ✓ Keybinding configuration             │
│  tools.rs:                               │
│    ✓ Tool command configuration           │
│    ✓ File manager adapter settings        │
│    ✓ AI, Git, Editor tool configs         │
└─────────────────────────────────────────┘
```

### 6. Resolution Layer (`src/resolve.rs`)

**Responsibility**: Executable path resolution.

```
┌─────────────────────────────────────────┐
│          Resolution Layer                  │
├─────────────────────────────────────────┤
│  ✓ Absolute path resolution               │
│  ✓ Executable lookup in PATH              │
│  ✓ Shell alias resolution via login shell│
│  ✓ Snap wrapper detection                │
│  ✓ Path canonicalization                  │
└─────────────────────────────────────────┘
```

---

## Action System Design

### Action Types

The system supports two primary action categories:

#### 1. File Manager Actions (`FileManagerAction`)

```
┌─────────────────────────────────────────────────────────────┐
│                    FileManagerAction Enum                        │
├─────────────────────────────────────────────────────────────┤
│  Open         │ Focus existing or create new file manager pane │
│  ToggleDock   │ Toggle between docked (left) and floating mode  │
│  Run         │ Internal: Execute the file manager process     │
└─────────────────────────────────────────────────────────────┘
```

#### 2. Terminal Actions (`TerminalAction`)

```
┌─────────────────────────────────────────────────────────────┐
│                      TerminalAction Enum                       │
├─────────────────────────────────────────────────────────────┤
│  Toggle     │ Hide terminal behind fullscreen editor or show   │
│  Zoom       │ Toggle terminal between docked height/fullscreen│
└─────────────────────────────────────────────────────────────┘
```

### Action Handler Pattern

The action handlers follow a consistent pattern:

1. **Resolve Zellij executable** - Get the path to the Zellij binary
2. **Query current state** - List panes to understand current workspace layout
3. **Find relevant panes** - Locate editor, terminal, file manager panes by name
4. **Determine action logic** - Branch based on current state and desired action
5. **Execute Zellij actions** - Send commands to Zellij to modify pane state
6. **Handle edge cases** - Manage scenarios where expected panes don't exist

---

## Data Flow Diagrams

### 1. Initialization Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  User Runs:   │────▶│  CLI Parse   │────▶│  Config Load │
│  sat-hx-ide   │     │              │     │              │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                             │
                                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    WorkspaceManager.init_workspace()            │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ Project Dir   │  │ Session Name  │  │ AI Availability Check │ │
│  │ Resolution   │  │ Generation    │  │ & Configuration        │ │
│  └──────────────┘  └──────────────┘  └─────────────────────┘ │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ Runtime Dir  │  │ Layout        │  │ Runtime Config       │ │
│  │ Creation     │  │ Generation    │  │ Generation           │ │
│  └──────────────┘  └──────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    ZellijClient Operations                      │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐ │
│  │ Session      │  │ Session       │  │ Session             │ │
│  │ Status       │─▶│ Delete (if    │─▶│ Create (if new/    │ │
│  │ Check        │  │ exited)       │  │ exited)             │ │
│  └──────────────┘  └──────────────┘  └─────────────────────┘ │
│  ┌──────────────┐                                              │
│  │ Attach       │←─────────────────────────────────────────────┘
│  │ (if active)  │
│  └──────────────┘
└─────────────────────────────────────────────────────────────┘
```

### 2. Keybinding Action Flow (File Manager Example)

```
┌──────────────┐
│ User Presses │
│ Ctrl+y       │
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    Zellij Keybinding Processing                   │
│  1. Zellij receives key event                                 │
│  2. Matches against keybindings in runtime config.kdl          │
│  3. Finds: bind "Ctrl y" { Run sat-hx-ide ... __file-manager open }│
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    sat-hx-ide Execution                          │
│  Command: sat-hx-ide --config /path/config.toml __file-manager open│
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Parsing & Dispatch                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ main.rs:Cli::parse()                                       │ │
│  │   ├── Recognizes hidden command __file-manager            │ │
│  │   └── Parses subcommand "open"                            │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ main.rs:138-146                                            │ │
│  │   match Commands::FileManager { action } =>               │ │
│  │     file_manager_action(&config, &config_path, action)   │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                file_manager_action() - actions.rs:46             │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 1. Resolve Zellij executable                            │ │
│  │ 2. List current panes (list_panes)                      │ │
│  │ 3. Find existing file-manager pane                      │ │
│  │ 4. Match on FileManagerAction::Open                    │ │
│  │    ├─ If pane exists: focus it                          │ │
│  │    └─ If not:                                           │ │
│  │        ├─ Get current pane ID                          │ │
│  │        ├─ Find editor pane                              │ │
│  │        ├─ Rename current pane to "file-manager"         │ │
│  │        └─ Run file manager (run_file_manager)          │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    run_file_manager() - actions.rs:168          │
│  1. Check adapter (currently only "yazi" supported)            │
│  2. Resolve file manager executable                           │
│  3. Create chooser file path in runtime dir                    │
│  4. Loop:                                                    │
│     a. Launch file manager with --chooser-file arg           │
│     b. Wait for user selection                              │
│     c. Read selections from chooser file                    │
│     d. Open first selection in editor                       │
│     e. Repeat until no selections or pane closed             │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    open_in_editor() - actions.rs:250              │
│  1. Find editor pane by name "editor"                         │
│  2. Quote the path for Helix (handle special chars)          │
│  3. Send keys to editor:                                    │
│     - Esc (exit any current mode)                           │
│     - :open <quoted-path>                                    │
│     - Enter                                                  │
└─────────────────────────────────────────────────────────────┘
```

### 3. Terminal Toggle Action Flow

```
┌──────────────┐
│ User Presses │
│ Alt+t        │
└──────┬───────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    Keybinding Processing                         │
│  bind "Alt t" { Run sat-hx-ide ... __terminal toggle }         │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    sat-hx-ide Execution                          │
│  Command: sat-hx-ide --config /path/config.toml __terminal toggle│
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    CLI Dispatch                                  │
│  main.rs:147-154 → terminal_action(&config, TerminalAction::Toggle)│
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                terminal_action() - actions.rs:291                 │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 1. Resolve Zellij executable                            │ │
│  │ 2. List current panes                                    │ │
│  │ 3. Find editor pane (required)                          │ │
│  │ 4. Find terminal pane (optional)                       │ │
│  │ 5. If terminal doesn't exist: respawn it               │ │
│  │ 6. Determine current state:                             │ │
│  │     - hidden = editor.is_fullscreen                    │ │
│  │     - zoomed = terminal.is_fullscreen                  │ │
│  │ 7. Match action and state:                              │ │
│  │     Case: Toggle when hidden                           │ │
│  │       ├─ toggle_fullscreen(editor) - show editor       │ │
│  │       └─ focus_pane(terminal) - focus terminal         │ │
│  │     Case: Toggle when not hidden                       │ │
│  │       ├─ If zoomed: toggle_fullscreen(terminal)        │ │
│  │       └─ toggle_fullscreen(editor) - hide editor        │ │
│  │     (similar logic for Zoom action)                    │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### 4. Configuration Generation Flow

```
┌─────────────────────────────────────────────────────────────┐
│              Configuration Loading & Merging                     │
├─────────────────────────────────────────────────────────────┤
│  User Config Loading                                          │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐ │
│  │ config.toml   │◄────┤ Config::load │◄────┤ Path from    │ │
│  │ (user-provided│     │              │     │ CLI or        │ │
│  │  or default)  │     │              │     │ default path  │ │
│  └──────────────┘     └──────────────┘     └──────────────┘ │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│              Runtime Config Generation                         │
│  build_runtime_config() - runtime_config.rs:21                 │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 1. Load user's Zellij config (if exists)                 │ │
│  │ 2. Validate no keybinding conflicts                       │ │
│  │ 3. Generate helper bindings for each action:              │ │
│  │    └─ file_manager key → __file-manager open             │ │
│  │    └─ file_manager_dock key → __file-manager toggle-dock  │ │
│  │    └─ terminal key → __terminal toggle                    │ │
│  │    └─ terminal_zoom key → __terminal zoom                 │ │
│  │    └─ git key → git client execution                    │ │
│  │ 4. Set session name and serialization off                │ │
│  │ 5. Merge with user's config                              │ │
│  │ 6. Write to runtime config.kdl                           │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────────┐
│                    Layout Generation                             │
│  session_layout() - layout.rs:4                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ 1. Create base layout with tabs                          │ │
│  │ 2. If status_bar enabled: add status_bar tab template    │ │
│  │ 3. Add "code" tab:                                        │ │
│  │    ├─ If terminal enabled: split with editor + terminal   │ │
│  │    └─ Otherwise: single editor pane                       │ │
│  │ 4. If AI enabled: add "ai" tab with AI client             │ │
│  │ 5. Generate KDL string                                     │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Key Design Decisions

### 1. Hidden CLI Commands for Keybindings

**Decision**: Use hidden CLI commands (`__file-manager`, `__terminal`) as callbacks for Zellij keybindings instead of implementing a Zellij plugin.

**Rationale**:
- Simpler implementation - leverages existing CLI infrastructure
- Consistent error handling through the same code paths
- Easy to debug - can manually trigger actions from terminal
- No need for complex plugin development or IPC
- Natural fit with Zellij's `Run` action which executes shell commands

### 2. Runtime Configuration Generation

**Decision**: Generate a runtime-specific Zellij config file instead of modifying the user's config.

**Rationale**:
- Preserves user's existing Zellij configuration
- Allows per-session keybindings and settings
- Clean separation between sat-hx-ide managed config and user config
- Easy cleanup - runtime files are in temporary directories
- Enables project-specific overrides

### 3. Process-Based File Manager Integration

**Decision**: Integrate with file managers (Yazi) through chooser files rather than direct IPC or plugin APIs.

**Rationale**:
- Yazi supports chooser files natively
- Simple file-based communication
- No need for complex inter-process communication
- Works with any file manager that supports this pattern
- Easy to extend to other file managers in the future

### 4. Pane Name-Based Discovery

**Decision**: Identify panes by their names (`editor`, `terminal`, `file-manager`) instead of IDs or other attributes.

**Rationale**:
- Predictable and stable - pane names are set during layout generation
- Works across sessions and restarts
- Easy to understand and debug
- Consistent with Zellij's pane management model
- Simplifies action handler logic

### 5. Fullscreen-Based Terminal Toggle

**Decision**: Use Zellij's fullscreen feature to hide/show terminal instead of closing/creating panes.

**Rationale**:
- Faster - no process creation/destruction
- Preserves terminal state and history
- Simple implementation using existing Zellij features
- Consistent user experience (terminal is always there, just hidden)
- Easy to extend with additional states (docked, fullscreen, hidden)

### 6. Helper Pane Pattern

**Decision**: Use floating, close-on-exit panes for action helpers instead of modifying existing panes.

**Rationale**:
- Clean separation - action helpers run in isolated panes
- Automatic cleanup - panes close when the helper process exits
- No interference with user's existing pane layout
- Consistent user experience - main workspace panes remain stable
- Easy to implement using Zellij's pane management features

---

## Pros and Cons

### Architecture Pros

| Advantage | Description |
|-----------|-------------|
| **Non-Invasive** | Doesn't modify any tool's configuration (Helix, Zellij, Yazi, etc.) - only generates runtime configs |
| **Composable** | Each component has a single responsibility, making it easy to extend or replace |
| **Debuggable** | Actions can be triggered manually from CLI for debugging |
| **Configurable** | All keybindings, commands, and behaviors are configurable via TOML |
| **Session-Aware** | Generates per-session configurations, enabling different setups per project |
| **Tool-Agnostic** | Can work with different editors, file managers, and AI clients |
| **Resilient** | Graceful handling of missing tools and edge cases |
| **Portable** | Pure Rust implementation with minimal dependencies |

### Architecture Cons

| Limitation | Description | Mitigation |
|------------|-------------|------------|
| **Process Overhead** | Each action spawns a new sat-hx-ide process | Actions are lightweight and fast |
| **Zellij Dependency** | Requires Zellij to be installed and configured | Clear error messages when not found |
| **Shell Environment** | Keybindings use shell-like syntax but run through Zellij | Good validation and error handling |
| **Single File Manager** | Currently only supports Yazi | Designed to be extensible to other managers |
| **Runtime Files** | Creates temporary files in runtime directory | Files are cleaned up automatically |
| **Limited IPC** | Communication between components is process-based | Sufficient for current use cases |

### Design Trade-offs

#### 1. Hidden Commands vs. Zellij Plugin

| Aspect | Hidden Commands | Zellij Plugin |
|--------|----------------|---------------|
| Implementation Complexity | Low | High |
| Development Time | Short | Long |
| Debugging | Easy (CLI) | Hard (plugin API) |
| Performance | Good | Better |
| Maintainability | High | Medium |
| **Chosen** | ✓ | |

#### 2. Runtime Config vs. User Config Modification

| Aspect | Runtime Config | User Config Mod |
|--------|----------------|------------------|
| User Impact | None | Modifies user files |
| Flexibility | Per-session | Global |
| Cleanup | Automatic | Manual |
| Complexity | Medium | Low |
| **Chosen** | ✓ | |

#### 3. Chooser Files vs. Direct Integration

| Aspect | Chooser Files | Direct Integration |
|--------|---------------|---------------------|
| Compatibility | Works with any FM | FM-specific |
| Complexity | Low | High |
| Reliability | High | Medium |
| Performance | Good | Better |
| **Chosen** | ✓ | |

---

## Component Relationships

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           sat-helix-ide Architecture                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐       │
│  │     CLI         │     │    Config        │     │    Resolve       │       │
│  │   (main.rs)     │     │   (config/)      │     │   (resolve.rs)   │       │
│  └────────┬────────┘     └────────┬────────┘     └────────┬────────┘       │
│           │                      │                      │                   │
│           │                      ▼                      │                   │
│           │              ┌─────────────────┐            │                   │
│           │              │  Workspace       │◄───────────┘                   │
│           │              │  Manager         │                             │
│           │              │ (workspace/)     │                             │
│           │              └────────┬────────┘                             │
│           │                       │                                       │
│           ▼                       ▼                                       │
│  ┌─────────────────┐     ┌─────────────────┐                               │
│  │   Actions        │     │    Zellij        │                               │
│  │  (actions.rs)    │     │  Integration     │                               │
│  └─────────────────┘     │  (zellij/)       │                               │
│                          └────────┬────────┘                               │
│                                   │                                         │
│                                   ▼                                         │
│                          ┌─────────────────┐                                 │
│                          │   Zellij Client  │                                 │
│                          │  (zellij/client) │                                 │
│                          └─────────────────┘                                 │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                           External Dependencies                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐    │
│  │   Zellij    │   │   Helix     │   │  Yazi       │   │  lazygit    │    │
│  │  (required) │   │  (required)  │   │ (optional)  │   │ (optional)  │    │
│  └─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘    │
│                                                                                 │
│  External Tools Used:                                                         │
│  - which: For executable resolution                                           │
│  - anyhow: For error handling                                                 │
│  - clap: For CLI argument parsing                                             │
│  - serde: For configuration serialization                                       │
│  - toml: For configuration file parsing                                        │
│  - kdl: For Zellij layout and config generation                                │
│  - home: For home directory resolution                                        │
│  - env_logger/log: For debug logging                                          │
│  - tempfile: For testing                                                      │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Dependency Flow

```
User Command
     │
     ▼
CLI (main.rs)
     │
     ├── Config Load → Config Layer (config/)
     │       │
     │       ├── Tool Config → Tool Resolution (resolve.rs)
     │       │
     │       └── Keybindings → Runtime Config Generation
     │
     ├── Workspace Init → Workspace Manager
     │       │
     │       ├── Zellij Integration → Zellij Client
     │       │       │
     │       │       ├── Session Management
     │       │       │
     │       │       ├── Layout Generation
     │       │       │
     │       │       └── Runtime Config Generation
     │       │
     │       └── Tool Checking → Resolve Layer
     │
     └── Action Execution → Actions Layer
             │
             ├── File Manager Actions → Zellij Pane Management
             │
             └── Terminal Actions → Zellij Pane Management
```

---

## Summary

sat-helix-ide follows a clean, modular architecture that:

1. **Separates concerns** - Each layer has a clear responsibility
2. **Uses composition** - Components work together through well-defined interfaces
3. **Embraces simplicity** - Leverages existing tools and simple patterns (files, process spawning)
4. **Prioritizes user experience** - Non-invasive, configurable, and debuggable
5. **Is extensible** - Designed to support additional tools and actions easily

The architecture's main strength is its simplicity and reliance on well-understood Unix patterns (processes, files, CLI arguments), making it robust and easy to maintain. The trade-off is some process overhead for action execution, but this is minimal compared to the benefits of a clean, debuggable system.
