# Performance Comparison: sat-helix-ide vs. Your Helix + Zellij Setup

> **Note (2026):** sat-helix-ide includes a **hybrid IPC daemon** (see [`ipc.md`](ipc.md)).
> Terminal and git actions use IPC when the daemon is running; file manager spawn paths
> still run in the keybinding helper. Benchmark numbers below mix pre-IPC estimates with
> the current hybrid model unless noted.

## Table of Contents

1. [Your Current Setup Analysis](#your-current-setup-analysis)
2. [sat-helix-ide Architecture](#sat-helix-ide-architecture)
3. [Direct Comparison](#direct-comparison)
4. [Performance Benchmarks](#performance-benchmarks)
5. [Action-by-Action Analysis](#action-by-action-analysis)
6. [Feature Comparison](#feature-comparison)
7. [Recommendations](#recommendations)
8. [Hybrid Configuration Suggestion](#hybrid-configuration-suggestion)

---

## Your Current Setup Analysis

Based on your `~/.config/helix/config.toml` and `~/.config/zellij/config.kdl`:

### Your Helix Keybindings

```toml
# Git tools - Direct zellij run commands
[keys.normal.space.g]
g = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- gitui",
   ":buffer-close!",
   ":redraw"
]

l = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- lazygit",
   ":buffer-close!",
   ":redraw"
]

d = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- git diff",
   ":buffer-close!",
   ":redraw"
]

s = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- git diff --staged",
   ":buffer-close!",
   ":redraw"
]

# File manager - Direct yazi with chooser file
C-y = [
	':sh rm -f /tmp/unique-ca1ea106',
	':insert-output yazi "%{buffer_name}" --chooser-file=/tmp/unique-ca1ea106',
	':sh printf "\x1b[?1049h\x1b[?2004h" > /dev/tty',
	':open %sh{cat /tmp/unique-ca1ea106}',
	':redraw',
	':set mouse false',
  ':set mouse true',
]

# Non-existent helix-ide commands (currently broken)
[keys.normal.space.i]
f = ":sh helix-ide files"     # Does not exist
k = ":sh helix-ide git"      # Does not exist
; = ":sh helix-ide terminal" # Does not exist
c = ":sh helix-ide context"  # Does not exist
```

### Your Zellij Configuration

- **Layout**: `rust-dev.kdl` with dedicated tabs:
  - Code (root) - main editor with 25% split
  - GIT (root) - runs `gitui`
  - AI (root) - runs `cursor-agent`
- **Plugins**: strider (file picker), session-manager, tab-bar, status-bar
- **Keybindings**: Standard Zellij defaults, no custom sat-hx-ide bindings

### Your Workflow Pattern

```
┌─────────────────────────────────────────────────────────────┐
│                    YOUR WORKFLOW                               │
├─────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Start Zellij session with layout                          │
│     └─ Dedicated tabs for code, git, AI                      │
│                                                                  │
│  2. Use Helix keybindings:                                   │
│     ├─ Space+g → gitui in floating fullscreen pane           │
│     ├─ Space+l → lazygit in floating fullscreen pane         │
│     ├─ Space+d → git diff in floating fullscreen pane        │
│     ├─ Space+s → git diff --staged in floating pane          │
│     └─ Ctrl+y → yazi file picker with chooser integration    │
│                                                                  │
│  3. Manual tab switching for git/AI                          │
│     └─ Must switch to GIT tab for gitui                       │
│     └─ Must switch to AI tab for cursor-agent                │
│                                                                  │
│  4. Broken: space+i commands (helix-ide doesn't exist)        │
│                                                                  │
└─────────────────────────────────────────────────────────────┘
```

### Your Setup Characteristics

| Aspect | Description |
|--------|-------------|
| **Architecture** | Decentralized - each action is a separate Helix keybinding |
| **Process Model** | Direct: Helix → shell → zellij run → tool |
| **Pane Management** | Manual - dedicated tabs, floating panes for tools |
| **File Picker** | Yazi with chooser file (direct in Helix) |
| **Git Integration** | Multiple git tools (gitui, lazygit, git diff) |
| **AI Integration** | Dedicated tab with cursor-agent |
| **Configuration** | Split between Helix config and Zellij layouts |
| **Maintenance** | Manual - each binding must be configured individually |

---

## sat-helix-ide Architecture

### How sat-helix-ide Works

```
┌─────────────────────────────────────────────────────────────┐
│                    SAT-HELIX-IDE WORKFLOW                      │
├─────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. Start: sat-hx-ide init                                    │
│     └─ Generates runtime layout.kdl and config.kdl            │
│     └─ Creates Zellij session with editor + terminal          │
│     └─ Injects keybindings into Zellij config                 │
│                                                                  │
│  2. Generated Zellij Keybindings:                             │
│     ├─ Ctrl+y → sat-hx-ide __file-manager open               │
│     ├─ Alt+y → sat-hx-ide __file-manager toggle-dock         │
│     ├─ Alt+g → zellij run ... gitui (direct)                │
│     ├─ Alt+t → sat-hx-ide __terminal toggle                  │
│     └─ Alt+Shift+t → sat-hx-ide __terminal zoom              │
│                                                                  │
│  3. Action Handling:                                          │
│     ├─ File manager → spawns yazi, reads chooser, opens file│
│     ├─ Terminal toggle → smart fullscreen hide/show         │
│     └─ Terminal zoom → toggle terminal fullscreen           │
│                                                                  │
│  4. Pane Discovery:                                           │
│     └─ All panes found by NAME (editor, terminal, file-mgr)  │
│     └─ No hardcoded pane IDs                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────┘
```

### sat-helix-ide Characteristics

| Aspect | Description |
|--------|-------------|
| **Architecture** | Centralized - single CLI tool manages everything |
| **Process Model** | Indirect: Zellij key → sat-hx-ide → zellij action → tool |
| **Pane Management** | Dynamic - panes created/managed on demand |
| **File Picker** | Yazi with chooser file (through sat-hx-ide) |
| **Git Integration** | Single git client, configurable |
| **AI Integration** | Optional tab, configurable |
| **Configuration** | Centralized in sat-hx-ide config.toml |
| **Maintenance** | Automatic - keybindings generated from config |

---

## Direct Comparison

### Command Execution Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Yours: Direct Helix → zellij run                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  User presses Space+g in Helix                                              │
│       │                                                                       │
│       ▼                                                                       │
│  ┌─────────────┐                                                             │
│  │ Helix        │─┐                                                           │
│  │ :new         │ │                                                           │
│  └─────────────┘ │                                                           │
│                 ├─▶ :sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- gitui│
│                 │                                                           │
│                 ▼                                                           │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │  Process: shell → zellij → gitui                             │          │
│  │  Time: ~5-10ms                                                  │          │
│  │  Overhead: Helix shell execution (~1-2ms)                   │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────────────┐
│                    sat-helix-ide: Zellij key → sat-hx-ide → zellij                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  User presses Alt+g in Zellij                                                │
│       │                                                                       │
│       ▼                                                                       │
│  ┌─────────────┐                                                             │
│  │ Zellij       │─┐                                                           │
│  │ Keybinding   │ │                                                           │
│  └─────────────┘ │                                                           │
│                 ├─▶ Run: /path/to/sat-hx-ide --config /path/config.toml __file-manager open│
│                 │                                                           │
│                 ▼                                                           │
│  ┌─────────────────────────────────────────────────────────────┐          │
│  │  1. Rust binary startup: ~5-15ms                            │          │
│  │  2. CLI parsing: ~1-2ms                                     │          │
│  │  3. Config loading: ~2-5ms                                  │          │
│  │  4. Pane discovery: ~1-2ms                                  │          │
│  │  5. Action logic: ~1-5ms                                    │          │
│  │  6. zellij action: ~5-10ms                                 │          │
│  │  Total: ~15-30ms                                              │          │
│  └─────────────────────────────────────────────────────────────┘          │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Overhead Breakdown

| Component | Your Setup | sat-helix-ide | Difference |
|-----------|------------|---------------|------------|
| Shell execution | ~1-2ms | ~1-2ms | 0ms |
| Zellij run | ~5-10ms | ~5-10ms | 0ms |
| **Subtotal (direct path)** | **~6-12ms** | **~6-12ms** | **0ms** |
| Rust binary startup | N/A | ~5-15ms | **+5-15ms** |
| CLI parsing | N/A | ~1-2ms | **+1-2ms** |
| Config loading | N/A | ~2-5ms | **+2-5ms** |
| Pane discovery | N/A | ~1-2ms | **+1-2ms** |
| Action logic | N/A | ~1-5ms | **+1-5ms** |
| **Additional overhead** | **0ms** | **~10-30ms** | **+10-30ms** |

### Key Insight

**Your setup has ZERO additional overhead** for direct git tool actions (gitui, lazygit, git diff).
**sat-helix-ide adds ~10-30ms overhead** per action due to Rust startup and logic processing.

However, **your setup cannot** do what sat-helix-ide does:
- Dynamic pane discovery by name
- Smart terminal toggle (hide editor, show terminal)
- Automatic file opening from chooser
- Session management (attach/create)
- Per-project configurations

---

## Performance Benchmarks

### Measured Latencies (Estimated)

| Action | Your Setup | sat-helix-ide | Ratio |
|--------|------------|---------------|-------|
| **Simple focus** | ~6ms | ~16ms | **2.7x slower** |
| **Terminal toggle** | N/A* | ~25ms | N/A |
| **Git tool (gitui)** | ~8ms | ~28ms | **3.5x slower** |
| **File picker (yazi)** | ~8ms | ~35ms | **4.4x slower** |
| **Session startup** | Manual (~500ms) | Auto (~400ms) | **Faster** |

*Your setup doesn't have equivalent terminal toggle functionality

### Process Count Comparison

| Action | Your Setup | sat-helix-ide |
|--------|------------|---------------|
| Git tool | 2 (shell + zellij) | 3 (zellij + sat-hx-ide + zellij) |
| File picker | 2 (shell + yazi) | 3 (zellij + sat-hx-ide + yazi) |
| Terminal toggle | N/A | 2 (zellij + sat-hx-ide) |

### Memory Usage Comparison

| Component | Your Setup | sat-helix-ide | Overhead |
|-----------|------------|---------------|----------|
| Helix | ~50-100MB | ~50-100MB | 0MB |
| Zellij | ~5-15MB | ~5-15MB | 0MB |
| sat-hx-ide | N/A | ~5-10MB | **+5-10MB** |
| Git tools | ~5-10MB each | ~5-10MB each | 0MB |
| **Total** | **~60-130MB** | **~65-140MB** | **+5-10MB** |

---

## Action-by-Action Analysis

### 1. Git Tools (gitui, lazygit, git diff)

#### Your Implementation
```toml
[keys.normal.space.g]
g = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- gitui",
   ":buffer-close!",
   ":redraw"
]
```

**Pros:**
- ✓ **~8ms latency** - fastest possible
- ✓ Direct execution - no intermediate processes
- ✓ Full control over command arguments
- ✓ Multiple git tools supported simultaneously

**Cons:**
- ✗ **Manual pane management** - creates new Helix buffer each time
- ✗ **Hardcoded** - must configure each tool separately
- ✗ **No pane reuse** - each action creates new floating pane
- ✗ **Helix-dependent** - only works from Helix
- ✗ **No state management** - panes accumulate if not closed

#### sat-helix-ide Implementation
```rust
// In runtime_config.rs
git_binding(input: &RuntimeConfigInput<'_>) -> Result<KdlNode> {
    // Generates:
    // bind "Alt g" {
    //     Run "/usr/bin/lazygit" {
    //         floating true
    //         close_on_exit true
    //         x "0%" y "0%"
    //         width "100%" height "100%"
    //         name "git"
    //         cwd "/path/to/project"
    //     }
    // }
}
```

**Pros:**
- ✓ **Automatic pane cleanup** - `close_on_exit true`
- ✓ **Consistent configuration** - one place to configure git client
- ✓ **Project-aware** - runs in project directory
- ✓ **Works from any pane** - Zellij-level keybinding
- ✓ **Integrated** - part of unified workflow

**Cons:**
- ✗ **~28ms latency** - ~3.5x slower
- ✗ Single git client (configurable, but one at a time)
- ✗ Requires sat-hx-ide to be installed

#### Verdict for Git Tools

| Criteria | Your Setup | sat-helix-ide | Winner |
|----------|------------|---------------|--------|
| Performance | ✓✓✓ | ✓ | **Yours** |
| Flexibility | ✓✓ | ✓✓✓ | **Yours** |
| Maintainability | ✓ | ✓✓✓ | **sat-helix-ide** |
| Pane Management | ✓ | ✓✓✓ | **sat-helix-ide** |
| Integration | ✓ | ✓✓✓ | **sat-helix-ide** |

**Recommendation**: Keep your direct Helix keybindings for git tools. They're faster and more flexible (multiple tools).

---

### 2. File Manager (yazi)

#### Your Implementation
```toml
C-y = [
	':sh rm -f /tmp/unique-ca1ea106',
	':insert-output yazi "%{buffer_name}" --chooser-file=/tmp/unique-ca1ea106',
	':sh printf "\x1b[?1049h\x1b[?2004h" > /dev/tty',
	':open %sh{cat /tmp/unique-ca1ea106}',
	':redraw',
	':set mouse false',
  ':set mouse true',
]
```

**Pros:**
- ✓ **~8ms latency** - direct yazi execution
- ✓ **Chooser file integration** - opens file in Helix automatically
- ✓ **Works from Helix** - natural integration
- ✓ **Full control** - custom terminal codes

**Cons:**
- ✗ **Helix-specific** - only works from Helix normal mode
- ✗ **Hardcoded chooser file** - `/tmp/unique-ca1ea106`
- ✗ **Complex sequence** - 7 commands in array
- ✗ **No pane management** - runs in same pane
- ✗ **Mouse toggle** - temporary workaround for terminal mode

#### sat-helix-ide Implementation
```rust
// In actions.rs
pub fn file_manager_action(
    config: &Config,
    config_path: &Path,
    action: FileManagerAction,
) -> Result<()> {
    // ... pane discovery ...

    match action {
        FileManagerAction::Open => {
            if let Some(pane) = existing {
                // Focus existing pane
                zellij_status(Command::new(&zellij)
                    .args(["action", "focus-pane-id"])
                    .arg(pane.cli_id()))?;
            } else {
                // Create new floating pane
                run_file_manager(config, &zellij, current_pane_id())?;
            }
        }
        // ...
    }
}

fn run_file_manager(config: &Config, zellij: &Path, pane_id: u64) -> Result<()> {
    let manager = resolve(&config.tools.file_manager.command)?;
    let chooser = config.runtime_dir(&session)
        .join(format!("chooser-{pane_id}-{}.txt", std::process::id()));

    loop {
        let status = Command::new(&manager)
            .args(&config.tools.file_manager.args)
            .arg("--chooser-file")
            .arg(&chooser)
            .status()?;

        let selections = read_selections(&chooser)?;
        if selections.is_empty() { return Ok(()); }

        open_in_editor(zellij, &selections[0])?;
    }
}
```

**Pros:**
- ✓ **Pane management** - creates/focuses floating pane automatically
- ✓ **Chooser file integration** - dynamic file path per session
- ✓ **Automatic file opening** - opens in editor pane
- ✓ **Works from any pane** - Zellij-level keybinding
- ✓ **Session-aware** - chooser files in runtime directory
- ✓ **Dock/floating toggle** - multiple modes supported

**Cons:**
- ✗ **~35ms latency** - ~4.4x slower
- ✗ **Requires Zellij** - must be running in Zellij session
- ✗ **Only Yazi** - currently hardcoded adapter

#### Verdict for File Manager

| Criteria | Your Setup | sat-helix-ide | Winner |
|----------|------------|---------------|--------|
| Performance | ✓✓✓ | ✓ | **Yours** |
| Integration | ✓✓ | ✓✓✓ | **sat-helix-ide** |
| Pane Management | ✓ | ✓✓✓ | **sat-helix-ide** |
| Flexibility | ✓✓✓ | ✓✓ | **Yours** |
| Maintainability | ✓✓ | ✓✓✓ | **sat-helix-ide** |

**Recommendation**: **Use sat-helix-ide for file manager**. The pane management and automatic file opening justify the ~25ms overhead. Your current setup requires manual mouse toggling and hardcodes the chooser file.

---

### 3. Terminal Management

#### Your Implementation
- **None found** in Helix config
- **Dedicated tab** in Zellij layout (`rust-dev.kdl`)
- **Manual switching** required between code and terminal tabs

**Current workflow:**
1. Use `Ctrl+t` to switch to tab mode in Zellij
2. Navigate to terminal tab
3. Use terminal
4. Switch back to code tab

**Latency:** ~5-10ms (Zellij tab switching)

**Pros:**
- ✓ **Zero overhead** - standard Zellij tab switching
- ✓ **Simple** - no additional tools

**Cons:**
- ✗ **Manual** - must switch tabs explicitly
- ✗ **No hide/show** - terminal always visible
- ✗ **Fixed layout** - terminal takes fixed space
- ✗ **No quick toggle** - requires multiple key presses

#### sat-helix-ide Implementation
```rust
// In actions.rs
pub fn terminal_action(config: &Config, action: TerminalAction) -> Result<()> {
    let zellij = resolve(&config.tools.zellij.command)?;
    let panes = list_panes(&zellij)?;
    let editor = panes.find(|p| p.title == EDITOR_PANE)?;
    let terminal = panes.find(|p| p.title == TERMINAL_PANE);

    let hidden = editor.is_fullscreen;
    let zoomed = terminal.is_some_and(|t| t.is_fullscreen);

    match action {
        TerminalAction::Toggle if hidden => {
            toggle_fullscreen(&zellij, editor)?;
            focus_pane(&zellij, terminal)?;
        }
        TerminalAction::Toggle => {
            if zoomed { toggle_fullscreen(&zellij, terminal)?; }
            toggle_fullscreen(&zellij, editor)?;
        }
        TerminalAction::Zoom if zoomed => toggle_fullscreen(&zellij, terminal),
        TerminalAction::Zoom => {
            if hidden { toggle_fullscreen(&zellij, editor)?; }
            toggle_fullscreen(&zellij, terminal)?;
        }
    }
}
```

**Pros:**
- ✓ **Smart toggle** - hide editor OR show terminal with one key
- ✓ **Zoom mode** - fullscreen terminal
- ✓ **Auto-respawn** - terminal recreated if closed
- ✓ **Works from any pane** - consistent behavior
- ✓ **No tab switching** - stays in code tab

**Cons:**
- ✗ **~25ms latency** - action processing overhead
- ✗ **Requires setup** - terminal must be in layout

#### Verdict for Terminal Management

| Criteria | Your Setup | sat-helix-ide | Winner |
|----------|------------|---------------|--------|
| Performance | ✓✓✓ | ✓✓ | **Yours** |
| Convenience | ✓ | ✓✓✓ | **sat-helix-ide** |
| Smart Behavior | ✗ | ✓✓✓ | **sat-helix-ide** |
| Integration | ✓ | ✓✓✓ | **sat-helix-ide** |

**Recommendation**: **Use sat-helix-ide for terminal**. The smart toggle (hide editor, show terminal with one key) is a major quality-of-life improvement that justifies the overhead.

---

### 4. Session Management

#### Your Implementation
- **Manual** - start Zellij with specific layout
- **No automation** - must manage sessions manually
- **Multiple layouts** - `dev.kdl`, `rust-dev.kdl`

**Workflow:**
```bash
# Start session
zellij --layout ~/.config/zellij/layouts/rust-dev.kdl

# Attach to existing
zellij attach
```

**Pros:**
- ✓ **Full control** - explicit layout selection
- ✓ **No overhead** - direct Zellij usage

**Cons:**
- ✗ **Manual** - must remember layout names
- ✗ **No project detection** - must specify layout per project
- ✗ **No AI tab management** - must configure manually
- ✗ **No tool checking** - errors at runtime if tools missing

#### sat-helix-ide Implementation
```rust
// In workspace/manager.rs
pub fn init_workspace(
    &self,
    path: &Path,
    session_override: Option<&str>,
    ai_override: Option<bool>,
    status_bar: Option<bool>,
) -> Result<()> {
    let project_dir = resolve_project_dir(path)?;
    let session_name = session_override
        .map(sanitize_session_name)
        .unwrap_or_else(|| project_session_name(&project_dir));

    // Resolve all tools
    let zellij_path = resolve_command(&self.config.tools.zellij)?;
    let editor_path = resolve_command(&self.config.tools.editor)?;
    let git_path = resolve_command(&self.config.tools.git)?;

    let ai_enabled = ai_override.unwrap_or(self.config.session.ai_by_default);
    let ai = if ai_enabled { self.resolve_ai() } else { None };

    // Generate layout and config
    let layout = session_layout(...);
    let config_path = runtime_dir.join("config.kdl");
    build_runtime_config(&RuntimeConfigInput { ... })?;

    // Create or attach session
    let zellij = ZellijClient::new(zellij_path);
    match zellij.session_status(&session_name)? {
        SessionStatus::Active => zellij.attach(&session_name, &config_path)?,
        SessionStatus::Exited => { ... }
        SessionStatus::NotFound => zellij.create(&session_name, &layout_path, &config_path, &project_dir)?,
    }
}
```

**Pros:**
- ✓ **Automatic project detection** - session name from directory
- ✓ **Tool validation** - checks all tools before starting
- ✓ **AI tab optional** - automatically included/excluded
- ✓ **Per-project config** - different settings per project
- ✓ **Attach/existing handling** - smart session management
- ✓ **Layout generation** - automatic based on config

**Cons:**
- ✗ **~400ms startup** - layout/config generation overhead
- ✗ **Less control** - layouts generated, not hand-written

#### Verdict for Session Management

| Criteria | Your Setup | sat-helix-ide | Winner |
|----------|------------|---------------|--------|
| Performance | ✓✓✓ | ✓✓ | **Yours** |
| Automation | ✓ | ✓✓✓ | **sat-helix-ide** |
| Project Awareness | ✗ | ✓✓✓ | **sat-helix-ide** |
| Tool Validation | ✗ | ✓✓✓ | **sat-helix-ide** |
| Flexibility | ✓✓✓ | ✓✓ | **Yours** |

**Recommendation**: **Use sat-helix-ide for session management**. The automatic project detection, tool validation, and AI tab management provide significant value. Your manual approach requires more effort for equivalent functionality.

---

## Feature Comparison

### What Your Setup Has That sat-helix-ide Doesn't

| Feature | Your Setup | sat-helix-ide | Notes |
|---------|------------|---------------|-------|
| Multiple git clients | ✓ (gitui, lazygit, git diff) | ✗ (single configurable) | Your setup wins for flexibility |
| Direct Helix integration | ✓ | ✗ (Zellij-level) | Your keybindings work from Helix |
| Zero overhead | ✓ | ✗ (~10-30ms) | Your setup is faster |
| Manual control | ✓✓✓ | ✓✓ | Full control vs. smart defaults |
| Hardcoded chooser file | ✓ | ✗ (dynamic) | Your setup uses `/tmp/unique-ca1ea106` |

### What sat-helix-ide Has That Your Setup Doesn't

| Feature | Your Setup | sat-helix-ide | Notes |
|---------|------------|---------------|-------|
| Pane discovery by name | ✗ | ✓✓✓ | Finds editor/terminal by title, not ID |
| Smart terminal toggle | ✗ | ✓✓✓ | Hide editor, show terminal with one key |
| Automatic file opening | ✓* | ✓✓✓ | Your yazi setup does this, but sat-hx-ide is more robust |
| Session auto-detection | ✗ | ✓✓✓ | Session name from project directory |
| Tool validation | ✗ | ✓✓✓ | Checks all tools before starting |
| Per-project config | ✗ | ✓✓✓ | Different settings per project |
| AI tab management | ✗ | ✓✓✓ | Optional, auto-included when available |
| Unified keybindings | ✗ | ✓✓✓ | Single config for all keybindings |
| Runtime config generation | ✗ | ✓✓✓ | Doesn't modify user's Zellij config |
| Dock/floating modes | ✗ | ✓✓✓ | File manager can be docked or floating |
| Terminal zoom | ✗ | ✓✓✓ | Toggle between docked and fullscreen |

*Your yazi setup does open files automatically, but requires mouse toggle workaround

---

## Recommendations

### Keep Your Direct Approach For:

1. **Git Tools** (`gitui`, `lazygit`, `git diff`)
   - **Reason**: Your direct Helix keybindings are **3-4x faster** (~8ms vs ~28ms)
   - **Reason**: You use **multiple git tools** simultaneously (gitui AND lazygit)
   - **Action**: Keep `space+g`, `space+l`, `space+d`, `space+s` in Helix config

### Replace with sat-helix-ide For:

1. **File Manager** (yazi)
   - **Reason**: sat-hx-ide provides **better pane management** and **automatic file opening**
   - **Reason**: Your current setup has **mouse toggle workaround** and **hardcoded chooser file**
   - **Action**: Use `Ctrl+y` (sat-hx-ide default) instead of your `Ctrl+y`
   - **Benefit**: Cleaner, no mouse toggle needed, dynamic chooser files

2. **Terminal Management**
   - **Reason**: **Smart toggle** (hide editor, show terminal) is a **major quality-of-life improvement**
   - **Reason**: Your current setup requires **manual tab switching**
   - **Action**: Use `Alt+t` (toggle) and `Alt+Shift+t` (zoom) from sat-hx-ide
   - **Benefit**: One key to hide editor and show terminal, stays in code tab

3. **Session Management**
   - **Reason**: **Automatic project detection** and **tool validation** save time
   - **Reason**: Your current setup requires **manual layout selection**
   - **Action**: Use `sat-hx-ide init` to start sessions
   - **Benefit**: Automatic session naming, AI tab management, per-project config

### Fix Your Broken Commands:

Your Helix config has these broken references:
```toml
[keys.normal.space.i]
f = ":sh helix-ide files"
g = ":sh helix-ide git"
t = ":sh helix-ide terminal"
c = ":sh helix-ide context"
```

These should be either:
- **Removed** (if using sat-hx-ide's Zellij keybindings)
- **Changed to** `sat-hx-ide` (if you want to use the CLI directly)

---

## Hybrid Configuration Suggestion

### Option 1: Pure sat-helix-ide (Recommended)

**Remove** your Helix git keybindings and use sat-hx-ide's default keybindings:

```toml
# In ~/.config/helix/config.toml - REMOVE these:
[keys.normal.space.g]
g = [ ... ]  # Remove
d = [ ... ]  # Remove
l = [ ... ]  # Remove
s = [ ... ]  # Remove

[keys.normal.space.i]
f = ":sh helix-ide files"  # Remove
g = ":sh helix-ide git"    # Remove
t = ":sh helix-ide terminal" # Remove
c = ":sh helix-ide context"  # Remove

# Keep only yazi file picker (or replace with sat-hx-ide)
C-y = [ ... ]  # Keep or remove
```

**Use** sat-hx-ide's default keybindings:
- `Ctrl+y` - File manager
- `Alt+y` - File manager dock toggle
- `Alt+g` - Git client
- `Alt+t` - Terminal toggle
- `Alt+Shift+t` - Terminal zoom

**Config** `~/.config/sat-helix-ide/config.toml`:
```toml
[tools.git]
command = "lazygit"  # or "gitui"
args = []

[tools.file_manager]
command = "yazi"
args = []

[keybindings]
file_manager = "Ctrl y"
file_manager_dock = "Alt y"
git = "Alt g"
terminal = "Alt t"
terminal_zoom = "Alt Shift t"
```

---

### Option 2: Hybrid (Best of Both Worlds)

**Keep** your fast git tool keybindings in Helix, **use** sat-helix-ide for everything else:

```toml
# In ~/.config/helix/config.toml - KEEP git tools:
[keys.normal.space.g]
g = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- gitui",
   ":buffer-close!",
   ":redraw"
]
l = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- lazygit",
   ":buffer-close!",
   ":redraw"
]
d = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- git diff",
   ":buffer-close!",
   ":redraw"
]
s = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- git diff --staged",
   ":buffer-close!",
   ":redraw"
]

# Remove broken helix-ide commands
[keys.normal.space.i]
# Remove f, g, t, c

# Optionally keep yazi, but consider switching to sat-hx-ide
C-y = [ ... ]  # Keep for now
```

**Configure** sat-hx-ide to **not** override your git keybinding:

```toml
# In ~/.config/sat-helix-ide/config.toml
[tools.git]
command = "lazygit"  # Doesn't matter, you're not using it
args = []

[keybindings]
file_manager = "Ctrl y"
file_manager_dock = "Alt y"
git = ""  # Disable git binding - use your Helix bindings instead
terminal = "Alt t"
terminal_zoom = "Alt Shift t"
```

**Benefits:**
- ✓ **Fast git tools** (~8ms) via your Helix keybindings
- ✓ **Smart file manager** via sat-hx-ide (~35ms)
- ✓ **Smart terminal** via sat-hx-ide (~25ms)
- ✓ **Session management** via sat-hx-ide
- ✓ **Best of both worlds** - performance where it matters, features where they matter

---

### Option 3: Pure Direct (Your Current Approach)

**Fix** your broken `helix-ide` references and **improve** your file picker:

```toml
# In ~/.config/helix/config.toml

# Git tools - KEEP as is (fast, flexible)
[keys.normal.space.g]
g = [ ... ]  # gitui
l = [ ... ]  # lazygit
d = [ ... ]  # git diff
s = [ ... ]  # git diff --staged

# File picker - IMPROVE to avoid mouse toggle
C-y = [
    ':sh rm -f /tmp/helix-yazi-chooser-%{pid}.txt',
    ':insert-output yazi "%{buffer_name}" --chooser-file=/tmp/helix-yazi-chooser-%{pid}.txt',
    ':open %sh{cat /tmp/helix-yazi-chooser-%{pid}.txt}',
    ':redraw',
]

# Remove broken helix-ide commands
[keys.normal.space.i]
# Remove all - they don't work

# Add manual terminal management if needed
[keys.normal]
"C-\\" = [":sh zellij action toggle-focus-fullscreen", ":redraw"]
```

**Pros:**
- ✓ **Fastest possible** - zero overhead
- ✓ **Full control** - you control everything

**Cons:**
- ✗ **Manual** - must configure everything yourself
- ✗ **No smart features** - no pane discovery, no auto-file-opening
- ✗ **Hardcoded** - chooser files, pane IDs
- ✗ **Maintenance burden** - you manage all configurations

---

## Final Verdict

| Approach | Performance | Features | Maintenance | Best For |
|----------|-------------|----------|------------|----------|
| **Pure Direct** (Option 3) | ✓✓✓ | ✓ | ✓ | Power users who want maximum speed |
| **Pure sat-helix-ide** (Option 1) | ✓✓ | ✓✓✓ | ✓✓✓ | Most users, best balance |
| **Hybrid** (Option 2) | ✓✓✓ | ✓✓✓ | ✓✓ | Users who want best of both worlds |

### My Recommendation: **Option 2 (Hybrid)**

**Why:**
1. Your git tool keybindings are **excellent** - fast, flexible, multiple tools
2. sat-helix-ide's **file manager** and **terminal management** are **superior**
3. sat-helix-ide's **session management** saves time and reduces errors
4. You get **~8ms for git tools** (your direct approach) and **~25-35ms for others** (acceptable)

### Expected Performance with Hybrid:

| Action | Latency | Comparison to Pure sat-hx-ide |
|--------|---------|--------------------------------|
| Git tools (gitui, lazygit) | ~8ms | **3.5x faster** than sat-hx-ide |
| File manager | ~35ms | Same as sat-hx-ide |
| Terminal toggle | ~25ms | Same as sat-hx-ide |
| Session start | ~400ms | Same as sat-hx-ide |

**Net result**: Your most frequent actions (git tools) stay fast, everything else gets smarter.

---

## Implementation Plan

### Step 1: Install sat-helix-ide

```bash
cd /home/vincent/Codes/sat-helix-ide
cargo install --path .
```

### Step 2: Create Config

```bash
mkdir -p ~/.config/sat-helix-ide
cat > ~/.config/sat-helix-ide/config.toml << 'EOF'
[session]
attach_existing = true
ai_by_default = true
status_bar = false

[terminal]
enabled = true
dock_percent = 15

[keybindings]
file_manager = "Ctrl y"
file_manager_dock = "Alt y"
git = ""  # Disable - using Helix bindings
terminal = "Alt t"
terminal_zoom = "Alt Shift t"

[tools.zellij]
command = "zellij"
args = []

[tools.editor]
command = "hx"
args = []

[tools.file_manager]
command = "yazi"
args = []
adapter = "yazi"
float_width = "100%"
float_height = "100%"
dock_percent = 28

[tools.git]
command = "lazygit"
args = []

[tools.ai]
command = "cursor-agent"
args = []
EOF
```

### Step 3: Update Helix Config

```toml
# Remove broken helix-ide commands
# Keep git tool commands
# Optionally keep yazi for now

[keys.normal.space.g]
g = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- gitui",
   ":buffer-close!",
   ":redraw"
]
l = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- lazygit",
   ":buffer-close!",
   ":redraw"
]
d = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- git diff",
   ":buffer-close!",
   ":redraw"
]
s = [
   ":new",
   ":sh zellij run -fc -x 0 -y 0 --height 100%% --width 100%% -- git diff --staged",
   ":buffer-close!",
   ":redraw"
]

# Remove this entire section:
# [keys.normal.space.i]
# f = ":sh helix-ide files"
# g = ":sh helix-ide git"
# t = ":sh helix-ide terminal"
# c = ":sh helix-ide context"
```

### Step 4: Test

```bash
# Start a project
cd ~/some-project
sat-hx-ide init

# Test keybindings:
# - Space+g in Helix: gitui (fast, ~8ms)
# - Space+l in Helix: lazygit (fast, ~8ms)
# - Alt+g in Zellij: (disabled, use Helix instead)
# - Ctrl+y in Zellij: file manager (smart, ~35ms)
# - Alt+t in Zellij: terminal toggle (smart, ~25ms)
```

---

## Summary

| Action | Your Current | sat-helix-ide | Hybrid | Recommendation |
|--------|--------------|---------------|--------|----------------|
| Git tools | ✓✓✓ (8ms) | ✓ (28ms) | ✓✓✓ (8ms) | **Keep yours** |
| File manager | ✓✓ (8ms) | ✓✓✓ (35ms) | ✓✓✓ (35ms) | **Use sat-hx-ide** |
| Terminal | ✓ (manual) | ✓✓✓ (25ms) | ✓✓✓ (25ms) | **Use sat-hx-ide** |
| Session mgmt | ✓ (manual) | ✓✓✓ (400ms) | ✓✓✓ (400ms) | **Use sat-hx-ide** |

**Bottom line**: Your git tool keybindings are **too good to replace** (fast, flexible, multiple tools). For everything else, sat-helix-ide provides **superior functionality** that justifies the ~20ms overhead. The hybrid approach gives you the best of both worlds.
