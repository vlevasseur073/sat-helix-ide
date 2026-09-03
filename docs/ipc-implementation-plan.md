# IPC Implementation Plan for sat-helix-ide

## Overview

This document outlines a **hybrid IPC implementation plan** for sat-helix-ide, adding optional Unix domain socket IPC to the existing process-per-action architecture. This approach leverages the proven design from helix-ide (`~/Codes/helix-ide`) while maintaining backward compatibility.

### Goals

1. **Reduce action latency** by 60-80% (from ~25-40ms to ~5-10ms per action)
2. **Maintain backward compatibility** - CLI still works without daemon
3. **Reuse helix-ide patterns** - Minimize reinvention, follow established practices
4. **Gradual migration** - Hybrid model allows incremental adoption

### Architecture Decision

**Hybrid Model**:
- **Primary path**: IPC to long-running daemon (for performance)
- **Fallback path**: Current process-per-action model (for compatibility)
- **Decision logic**: Try IPC first, fallback to process if daemon not available

```
┌─────────────────────────────────────────────────────────────────┐
│                    HYBRID ARCHITECTURE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                     │
│  CLI Command Flow:                                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  1. Parse arguments                                             │    │
│  │  2. Check if daemon socket exists                             │    │
│  │  3. Try IPC request to daemon                                 │    │
│  │     ├─ Success: Return response                                │    │
│  │     └─ Failure: Fall back to process model                      │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Daemon (New):                                                     │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  - Listens on Unix domain socket                              │    │
│  │  - Loads config once at startup                               │    │
│  │  - Caches pane state with TTL                                │    │
│  │  - Handles all action requests                                │    │
│  │  - Manages Zellij connection pool                            │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                     │
│  Process Model (Existing):                                         │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  - Unchanged from current implementation                      │    │
│  │  - Used as fallback when daemon not available                 │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                     │
└─────────────────────────────────────────────────────────────────┘
```

---

## Reference: helix-ide Implementation

The helix-ide source code (`~/Codes/helix-ide`) provides a proven reference implementation:

### Key Files and Patterns

| File | Purpose | Key Patterns to Reuse |
|------|---------|----------------------|
| `src/ipc.rs` | IPC server/client | Unix socket binding, async connection handling, JSON protocol |
| `src/protocol.rs` | Message types | Tagged enums with serde, Request/Response pattern |
| `src/main.rs` | Daemon lifecycle | Daemon spawning, liveness checking, command routing |
| `src/app.rs` | Request handling | Arc<App> for shared state, request dispatch |

### helix-ide Protocol

```rust
// Request types
#[serde(tag = "command")]
pub enum Request {
    Open { component: Component },
    Context,
    Ping,
}

// Response types
#[serde(tag = "status")]
pub enum Response {
    Ok,
    Context { cwd, project_root, git_root },
    Error { message: String },
    Pong,
}
```

### helix-ide IPC Server

```rust
pub async fn serve(socket: &PathBuf, app: Arc<App>) -> Result<()> {
    let listener = UnixListener::bind(socket)?;
    loop {
        let (stream, _) = listener.accept().await?;
        let app = Arc::clone(&app);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, app).await {
                eprintln!("IPC error: {e:#}");
            }
        });
    }
}
```

### helix-ide IPC Client

```rust
pub async fn send(socket: &PathBuf, request: &Request) -> Result<Response> {
    let stream = UnixStream::connect(socket).await?;
    let (reader, mut writer) = stream.into_split();
    let mut message = serde_json::to_vec(request)?;
    message.push(b'\n');
    writer.write_all(&message).await?;
    let mut response = String::new();
    BufReader::new(reader).read_line(&mut response).await?;
    Ok(serde_json::from_str(&response)?)
}
```

---

## Implementation Plan

### Phase 1: Preparation (1 week)

#### 1.1 Add Required Dependencies

**File**: `Cargo.toml`

```toml
[dependencies]
# Existing dependencies...

# New dependencies for IPC
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "time"] }
```

**Rationale**: tokio provides async runtime and Unix socket support. We use the same features as helix-ide.

#### 1.2 Create Directory Structure

```bash
# New files to create
src/
├── ipc.rs              # IPC server and client
├── protocol.rs         # Request and Response types
└── daemon.rs           # Daemon lifecycle and request handling
```

#### 1.3 Update Main Entry Point

**File**: `src/main.rs`

Add async main and daemon command:

```rust
use tokio;

#[derive(Parser, Debug)]
#[command(name = "sat-hx-ide")]
struct Cli {
    // ... existing fields ...
}

#[derive(Subcommand, Debug)]
enum Commands {
    // ... existing commands ...

    #[command(hide = true)]
    Daemon,  // NEW: Daemon mode
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or_default() {
        // ... existing command handling ...

        Commands::Daemon => {
            daemon::run_daemon().await?;
        }
    }

    Ok(())
}
```

---

### Phase 2: Protocol Definition (3-5 days)

#### 2.1 Define Request/Response Types

**File**: `src/protocol.rs`

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Requests that can be sent to the daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
    // File manager actions
    FileManagerOpen,
    FileManagerToggleDock,
    FileManagerRun,

    // Terminal actions
    TerminalToggle,
    TerminalZoom,

    // State queries
    GetPaneList,
    GetContext,

    // Lifecycle
    Ping,
    Shutdown,
}

/// Responses from the daemon
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok,

    // Data responses
    PaneList(Vec<crate::actions::PaneInfo>),
    Context {
        cwd: PathBuf,
        project_root: PathBuf,
        git_root: Option<PathBuf>,
    },

    // Error response
    Error { message: String },

    // Lifecycle
    Pong,
}
```

**Rationale**:
- Tagged enums allow polymorphic serialization/deserialization
- `snake_case` renaming matches JSON conventions
- Separate Request and Response types for clarity
- Includes all current action types plus state queries

#### 2.2 Define Socket Path Convention

**File**: `src/ipc.rs`

```rust
use std::path::PathBuf;

/// Generate socket path for a given session name
///
/// Socket naming convention: /run/user/<uid>/sat-hx-ide-<session_hash>.sock
/// This follows helix-ide's pattern but with sat-hx-ide naming.
pub fn socket_path(session_name: &str) -> PathBuf {
    // Hash the session name to avoid filesystem issues with special characters
    let hash = session_name
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));

    // Use XDG_RUNTIME_DIR if available, otherwise /tmp
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir());

    runtime_dir.join(format!("sat-hx-ide-{hash}.sock"))
}

/// Generate socket path for the current session
/// Uses ZELLIJ_SESSION_NAME env var if available, otherwise defaults
pub fn current_socket_path() -> PathBuf {
    let session_name = std::env::var("ZELLIJ_SESSION_NAME")
        .unwrap_or_else(|_| "default".to_string());
    socket_path(&session_name)
}
```

---

### Phase 3: IPC Layer Implementation (1 week)

#### 3.1 IPC Server

**File**: `src/ipc.rs`

```rust
use crate::protocol::{Request, Response};
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Start the IPC server on the given socket
pub async fn serve(
    socket: &PathBuf,
    app: std::sync::Arc<crate::app::App>,
) -> Result<()> {
    // Remove stale socket if it exists
    if socket.exists() {
        std::fs::remove_file(socket)
            .with_context(|| format!("failed to remove stale socket {}", socket.display()))?;
    }

    // Create listener
    let listener = UnixListener::bind(socket)
        .with_context(|| format!("failed to bind socket {}", socket.display()))?;

    log::info!("IPC daemon listening on {}", socket.display());

    // Accept loop
    loop {
        let (stream, _) = listener.accept().await
            .with_context(|| "failed to accept IPC connection")?;

        let app = std::sync::Arc::clone(&app);

        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream, app).await {
                log::error!("IPC error: {error:#}");
            }
        });
    }
}

/// Handle a single IPC connection
async fn handle_connection(
    stream: UnixStream,
    app: std::sync::Arc<crate::app::App>,
) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read request (line-delimited JSON)
    let mut line = String::new();
    reader.read_line(&mut line).await
        .with_context(|| "failed to read IPC request")?;

    // Parse request
    let request: Request = serde_json::from_str(&line)
        .with_context(|| format!("failed to parse IPC request: {line}"))?;

    // Handle request
    let response = app.handle_ipc(request).await;

    // Send response
    let mut serialized = serde_json::to_vec(&response)?;
    serialized.push(b'\n');
    writer.write_all(&serialized).await
        .with_context(|| "failed to write IPC response")?;

    Ok(())
}

/// Send a request to the IPC daemon
pub async fn send_request(
    socket: &PathBuf,
    request: &Request,
) -> Result<Response> {
    let stream = UnixStream::connect(socket)
        .await
        .with_context(|| format!("cannot connect to daemon at {}", socket.display()))?;

    let (reader, mut writer) = stream.into_split();

    // Send request
    let mut message = serde_json::to_vec(request)?;
    message.push(b'\n');
    writer.write_all(&message).await
        .with_context(|| "failed to write IPC request")?;

    // Read response
    let mut reader = BufReader::new(reader);
    let mut response = String::new();
    reader.read_line(&mut response).await
        .with_context(|| "failed to read IPC response")?;

    // Parse response
    serde_json::from_str(&response)
        .with_context(|| format!("failed to parse IPC response: {response}"))
}

/// Check if the daemon is alive and responsive
pub async fn is_daemon_alive(socket: &PathBuf) -> bool {
    UnixStream::connect(socket).await.is_ok()
}

/// Spawn the daemon process
pub fn spawn_daemon(executable: &std::path::Path, socket: &PathBuf) -> Result<()> {
    let mut command = std::process::Command::new(executable);
    command
        .arg("daemon")
        .arg("--socket")
        .arg(socket);

    command.spawn()
        .with_context(|| "failed to spawn daemon")?;

    Ok(())
}
```

#### 3.2 Daemon Lifecycle

**File**: `src/daemon.rs`

```rust
use crate::app::App;
use crate::config::Config;
use crate::ipc::{self, serve};
use anyhow::{Context, Result};
use std::sync::Arc;

/// Daemon state
pub struct Daemon {
    app: Arc<App>,
}

impl Daemon {
    pub fn new(config: &Config) -> Result<Self> {
        let app = Arc::new(App::new(config));
        Ok(Self { app })
    }

    pub async fn run(&self) -> Result<()> {
        let socket = ipc::current_socket_path();
        serve(&socket, Arc::clone(&self.app)).await
    }
}

/// Run the daemon
pub async fn run_daemon() -> Result<()> {
    let config = Config::load(&std::env::var("SAT_HX_IDE_CONFIG")
        .unwrap_or_else(|_| "~/.config/sat-helix-ide/config.toml".into()))?;

    let daemon = Daemon::new(&config)?;
    daemon.run().await
}
```

---

### Phase 4: Daemon Integration (1 week)

#### 4.1 Create App State for Daemon

**File**: `src/app.rs` (new file)

```rust
use crate::config::Config;
use crate::actions::{FileManagerAction, TerminalAction, PaneInfo};
use crate::protocol::{Request, Response};
use anyhow::Result;
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;

/// Shared application state for the daemon
pub struct App {
    config: Config,
    /// Cache of pane information per session
    pane_cache: tokio::sync::RwLock<HashMap<String, Vec<PaneInfo>>>,
    /// Cache of resolved tool paths
    tool_cache: tokio::sync::RwLock<HashMap<String, PathBuf>>,
}

impl App {
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
            pane_cache: tokio::sync::RwLock::new(HashMap::new()),
            tool_cache: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    pub async fn handle_ipc(&self, request: Request) -> Response {
        match request {
            Request::FileManagerOpen => {
                self.handle_file_manager_open().await
            }
            Request::FileManagerToggleDock => {
                self.handle_file_manager_toggle_dock().await
            }
            Request::FileManagerRun => {
                self.handle_file_manager_run().await
            }
            Request::TerminalToggle => {
                self.handle_terminal_toggle().await
            }
            Request::TerminalZoom => {
                self.handle_terminal_zoom().await
            }
            Request::GetPaneList => {
                self.get_pane_list().await
            }
            Request::GetContext => {
                self.get_context().await
            }
            Request::Ping => Response::Pong,
            Request::Shutdown => {
                // Handle shutdown
                Response::Ok
            }
        }
    }

    // Handler implementations will go here
    // They will reuse the existing logic from actions.rs
}
```

#### 4.2 Update Action Handlers for Daemon

The daemon needs its own versions of the action handlers that:
- Use the cached pane list
- Return responses instead of directly executing
- Work in async context

**Example: Terminal Toggle in Daemon**

```rust
impl App {
    async fn handle_terminal_toggle(&self) -> Response {
        let zellij = self.resolved_tool("zellij").await?;
        let panes = self.get_panes(&zellij).await?;

        let editor = panes.iter()
            .find(|p| !p.is_plugin && p.title == "editor")
            .ok_or_else(|| Response::Error {
                message: "Cannot find editor pane".to_string()
            })?;

        let terminal = panes.iter()
            .find(|p| !p.is_plugin && p.title == "terminal");

        let hidden = editor.is_fullscreen;
        let zoomed = terminal.is_some_and(|t| t.is_fullscreen);

        // Execute the appropriate Zellij actions
        if hidden {
            self.toggle_fullscreen(&zellij, editor).await?;
            if let Some(terminal) = terminal {
                self.focus_pane(&zellij, terminal).await?;
            }
        } else {
            if zoomed {
                self.toggle_fullscreen(&zellij, terminal.unwrap()).await?;
            }
            self.toggle_fullscreen(&zellij, editor).await?;
        }

        // Invalidate pane cache after state change
        self.invalidate_pane_cache().await;

        Response::Ok
    }
}
```

---

### Phase 5: CLI Integration (3-5 days)

#### 5.1 Add Hybrid Request Logic

**File**: `src/main.rs` (updates)

```rust
use crate::ipc;
use crate::protocol::{Request, Response};

// ... existing code ...

/// Try to execute an action via IPC, falling back to process model
async fn try_ipc_or_process(
    command: Commands,
    config: &Config,
    config_path: &Path,
) -> Result<()> {
    // Check if daemon is running
    let socket = ipc::current_socket_path();
    if ipc::is_daemon_alive(&socket).await {
        // Convert command to IPC request
        let request = command_to_request(&command)?;

        // Send request
        match ipc::send_request(&socket, &request).await {
            Ok(Response::Ok) => return Ok(()),
            Ok(Response::Error { message }) => {
                log::warn!("IPC error: {}", message);
            }
            Ok(other) => {
                log::warn!("Unexpected IPC response: {:?}", other);
            }
            Err(e) => {
                log::warn!("IPC connection error: {}", e);
            }
        }
    }

    // Fallback to process model
    execute_command_process(command, config, config_path)
}

/// Convert CLI command to IPC request
fn command_to_request(command: &Commands) -> Result<Request> {
    match command {
        Commands::FileManager { action } => match action {
            FileManagerCommands::Open => Ok(Request::FileManagerOpen),
            FileManagerCommands::ToggleDock => Ok(Request::FileManagerToggleDock),
            FileManagerCommands::Run => Ok(Request::FileManagerRun),
        },
        Commands::Terminal { action } => match action {
            TerminalCommands::Toggle => Ok(Request::TerminalToggle),
            TerminalCommands::Zoom => Ok(Request::TerminalZoom),
        },
        // Other commands don't use IPC
        _ => Err(anyhow::anyhow!("Command not supported via IPC"))
    }
}

/// Execute command using process model (existing logic)
fn execute_command_process(
    command: Commands,
    config: &Config,
    config_path: &Path,
) -> Result<()> {
    // Existing logic from main.rs
    match command {
        // ... existing match arms ...
    }
}
```

#### 5.2 Update Main Command Flow

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        std::env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();

    match cli.command.unwrap_or_default() {
        Commands::Daemon => {
            daemon::run_daemon().await?;
        }
        Commands::FileManager { action } => {
            let config = Config::load(&cli.config)?;
            try_ipc_or_process(
                Commands::FileManager { action },
                &config,
                &cli.config,
            ).await?;
        }
        Commands::Terminal { action } => {
            let config = Config::load(&cli.config)?;
            try_ipc_or_process(
                Commands::Terminal { action },
                &config,
                &cli.config,
            ).await?;
        }
        // Other commands use process model directly
        Commands::Init { path, ai, no_ai, status_bar, session } => {
            let config = Config::load(&cli.config)?;
            let ai_override = match (ai, no_ai) {
                (true, _) => Some(true),
                (_, true) => Some(false),
                _ => None,
            };
            WorkspaceManager::new(&config, &cli.config)
                .init_workspace(&path, session.as_deref(), ai_override, status_bar)?;
        }
        // ... other commands ...
    }

    Ok(())
}
```

---

### Phase 6: Caching Layer (3-5 days)

#### 6.1 Pane Cache in Daemon

The daemon maintains a cache of pane information to avoid repeated `list-panes` calls:

```rust
impl App {
    /// Get panes for a session, using cache if fresh
    pub async fn get_panes(&self, zellij: &Path) -> Result<Vec<PaneInfo>> {
        use std::time::{Instant, Duration};

        // Generate cache key based on session
        let session_name = std::env::var("ZELLIJ_SESSION_NAME")
            .unwrap_or_else(|_| "default".to_string());

        // Check cache
        {
            let cache = self.pane_cache.read().await;
            if let Some(panes) = cache.get(&session_name) {
                return Ok(panes.clone());
            }
        }

        // Fetch fresh
        let panes = self.fetch_panes(zellij).await?;

        // Update cache
        {
            let mut cache = self.pane_cache.write().await;
            cache.insert(session_name, panes.clone());
        }

        Ok(panes)
    }

    /// Invalidate pane cache for current session
    pub async fn invalidate_pane_cache(&self) {
        let session_name = std::env::var("ZELLIJ_SESSION_NAME")
            .unwrap_or_else(|_| "default".to_string());

        let mut cache = self.pane_cache.write().await;
        cache.remove(&session_name);
    }

    /// Fetch panes from Zellij
    async fn fetch_panes(&self, zellij: &Path) -> Result<Vec<PaneInfo>> {
        let output = tokio::process::Command::new(zellij)
            .args(["action", "list-panes", "--json", "--all"])
            .output()
            .await
            .with_context(|| "Failed to query Zellij panes")?;

        if !output.status.success() {
            anyhow::bail!(
                "Failed to query Zellij panes: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        serde_json::from_slice(&output.stdout)
            .with_context(|| "Invalid pane JSON returned by Zellij")
    }
}
```

#### 6.2 Tool Path Cache

```rust
impl App {
    /// Get resolved tool path, using cache
    pub async fn resolved_tool(&self, tool_name: &str) -> Result<PathBuf> {
        // Check cache
        {
            let cache = self.tool_cache.read().await;
            if let Some(path) = cache.get(tool_name) {
                return Ok(path.clone());
            }
        }

        // Resolve path
        let path = crate::resolve::resolve_executable(tool_name)?;

        // Update cache
        {
            let mut cache = self.tool_cache.write().await;
            cache.insert(tool_name.to_string(), path.clone());
        }

        Ok(path)
    }
}
```

---

## Testing Strategy

### Unit Tests

1. **IPC Protocol Serialization**
   - Test Request/Response serialization/deserialization
   - Test tagged enum handling

2. **Socket Path Generation**
   - Test socket path with various session names
   - Test special character handling

3. **Pane Cache**
   - Test cache hit/miss
   - Test cache invalidation

### Integration Tests

1. **Daemon Lifecycle**
   - Test daemon startup
   - Test daemon shutdown
   - Test daemon restart

2. **IPC Communication**
   - Test request/response roundtrip
   - Test concurrent requests
   - Test error handling

3. **Fallback Behavior**
   - Test process model when daemon not running
   - Test graceful degradation

### End-to-End Tests

1. **Performance Benchmarks**
   - Measure action latency with/without daemon
   - Verify 3-5x improvement

2. **Functional Tests**
   - Test all actions via IPC
   - Test all actions via fallback
   - Test mixed usage

---

## Migration Path

### Step 1: Feature Flag

Add a feature flag to control IPC availability:

```toml
[features]
default = []
ipc = ["tokio"]
```

This allows:
- Building without tokio for minimal dependencies
- Testing IPC separately

### Step 2: Opt-In IPC

Initially, IPC is **opt-in**:
- Users must explicitly enable it via config or environment variable
- Default behavior remains process-per-action

```toml
[session]
ipc_enabled = false  # Default: false for backward compatibility
```

### Step 3: Opt-Out IPC

Later, make IPC **default** with opt-out:
- IPC becomes the default
- Users can disable it if needed

### Step 4: Full Migration

Finally, remove process-per-action code path:
- IPC becomes mandatory
- Clean up fallback logic

---

## Risk Mitigation

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Async runtime bugs | Low | High | Use proven tokio patterns from helix-ide |
| Socket conflicts | Low | Medium | Use session-specific socket names, cleanup stale sockets |
| Memory leaks | Low | High | Use Arc properly, limit cache sizes |
| Daemon crashes | Medium | Medium | Implement auto-restart, graceful fallback |
| Protocol errors | Low | Medium | Comprehensive error handling, logging |

### Business Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Breaking changes | Medium | High | Feature flag, opt-in initially, thorough testing |
| Performance regression | Low | High | Benchmark before/after, profile |
| Increased complexity | Medium | Medium | Clear architecture, good documentation |
| User confusion | Medium | Medium | Clear docs, migration guide |

---

## Success Criteria

### Phase 1: Preparation
- [x] Dependencies added (tokio, serde, serde_json, libc)
- [x] Directory structure created
- [x] Main entry point updated for async

### Phase 2: Protocol Definition
- [x] Request/Response types defined
- [x] Socket path convention implemented (project-based hashing)
- [x] Serialization/deserialization tested

### Phase 3: IPC Layer
- [x] IPC server implemented with graceful shutdown
- [x] IPC client implemented
- [x] Connection handling tested
- [x] Ping/pong health check implemented

### Phase 4: Daemon Integration
- [x] Daemon state created (Arc<App>)
- [x] Request handlers implemented
- [x] Error handling comprehensive
- [x] **NEW: Daemon persistence across session lifetime**
- [x] **NEW: PID file management for daemon tracking**
- [x] **NEW: Graceful shutdown with SIGTERM/SIGKILL**
- [x] **NEW: Automatic daemon cleanup when Zellij session ends**

### Phase 5: CLI Integration
- [x] Hybrid request logic implemented (IPC with fallback)
- [x] Fallback behavior tested
- [x] All actions work via both paths
- [x] Daemon auto-spawned during init_workspace()

### Phase 6: Caching Layer
- [x] Pane cache implemented (with 500ms TTL)
- [x] Tool cache implemented
- [x] Cache invalidation working

### Testing
- [ ] Unit tests pass (needs verification)
- [ ] Integration tests pass (needs manual testing)
- [ ] Performance benchmarks show improvement (pending)
- [x] No regressions in existing functionality (process model still works)

---

## Timeline

| Phase | Duration | Dependencies | Priority | Status |
|-------|----------|--------------|----------|--------|
| 1. Preparation | 1 week | None | High | ✅ Complete |
| 2. Protocol | 3-5 days | Phase 1 | High | ✅ Complete |
| 3. IPC Layer | 1 week | Phase 2 | High | ✅ Complete |
| 4. Daemon Integration | 1 week | Phase 3 | High | ✅ Complete |
| 5. CLI Integration | 3-5 days | Phase 4 | High | ✅ Complete |
| 6. Caching | 3-5 days | Phase 4 | Medium | ✅ Complete |
| **Daemon Lifecycle** | **2 days** | Phase 4 | High | ✅ **NEW: Complete** |
| Testing | 2 weeks | All phases | High | ⏳ Pending |
| **Total** | **6-8 weeks** | | | **Core Complete** |

**Note**: Core IPC implementation is complete. Remaining work focuses on testing, performance validation, and edge case handling.

---

## Code Reuse from helix-ide

The following patterns from helix-ide should be directly reused:

### 1. Socket Path Generation
```rust
// From helix-ide/src/ipc.rs
let hash = workspace.to_string_lossy().bytes()
    .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
```

### 2. Async Connection Handling
```rust
// From helix-ide/src/ipc.rs
let (stream, _) = listener.accept().await?;
tokio::spawn(async move { ... });
```

### 3. Line-Delimited JSON Protocol
```rust
// From helix-ide/src/ipc.rs
message.push(b'\n');
reader.read_line(&mut line).await?;
```

### 4. Tagged Enum Serialization
```rust
// From helix-ide/src/protocol.rs
#[serde(tag = "command")]
pub enum Request { ... }

#[serde(tag = "status")]
pub enum Response { ... }
```

### 5. Arc<App> for Shared State
```rust
// From helix-ide/src/main.rs and app.rs
let app = Arc::new(App::load()?);
tokio::spawn(async move { handle_connection(stream, app).await });
```

---

## Configuration Changes

### New Configuration Options

**File**: Default config (bundled) and user configs

```toml
[session]
# Enable IPC daemon for better performance (optional, default: false)
ipc_enabled = false

# Socket path override (optional, default: auto-generated)
# ipc_socket = "/run/user/1000/sat-hx-ide.sock"

# Daemon timeout in seconds (optional, default: 60)
# daemon_timeout = 60

# Pane cache TTL in milliseconds (optional, default: 500)
# pane_cache_ttl = 500
```

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `SAT_HX_IDE_IPC` | Enable/disable IPC | `auto` (try IPC, fallback to process) |
| `SAT_HX_IDE_SOCKET` | Override socket path | Auto-generated from session name |
| `SAT_HX_IDE_CONFIG` | Config file path | `~/.config/sat-helix-ide/config.toml` |

---

## Documentation Requirements

### User Documentation

1. **IPC Feature Overview**
   - What is IPC?
   - Benefits of IPC
   - When to enable/disable

2. **Configuration Guide**
   - How to enable IPC
   - Socket path configuration
   - Troubleshooting

3. **Migration Guide**
   - From process model to IPC
   - Backward compatibility notes
   - Performance expectations

### Developer Documentation

1. **Architecture Overview**
   - Component diagram
   - Data flow diagram
   - Sequence diagrams

2. **Protocol Specification**
   - Request types
   - Response types
   - Error handling
   - Versioning

3. **API Documentation**
   - IPC module
   - Protocol module
   - Daemon module

---

## Next Steps

1. **Review and approve** this implementation plan
2. **Create feature branch** for IPC development
3. **Start with Phase 1** (Preparation)
4. **Schedule regular check-ins** to track progress
5. **Plan for testing** resources and infrastructure

---

## Appendix: Complete File List

### New Files
- `src/ipc.rs` - IPC server and client
- `src/protocol.rs` - Request and Response types
- `src/daemon.rs` - Daemon lifecycle and state
- `src/app.rs` - Application state for daemon

### Modified Files
- `Cargo.toml` - Add tokio dependency
- `src/main.rs` - Add async main, daemon command, hybrid request logic
- `src/actions.rs` - Minor adjustments for daemon compatibility

### Configuration Files
- `configs/config.toml` - Add IPC configuration options

### Documentation Files
- `docs/ipc.md` - IPC feature documentation (new)
- `README.md` - Update with IPC information
