# IPC Daemon Guide

sat-hx-ide uses a **hybrid IPC architecture**: keybindings still spawn a short-lived
`sat-hx-ide` helper process, but action execution is delegated to a long-running
daemon when available. If the daemon is missing or cannot handle a request, the
helper falls back to the original process-per-action path in `actions.rs`.

## How it works

```
Zellij keybinding
    → sat-hx-ide __terminal toggle   (helper process)
        → Unix socket JSON request
            → daemon (cached panes/tools)
                → zellij action ...
        → fallback: actions.rs (sync)
```

During `sat-hx-ide init`, a daemon is started **before** Zellij and stopped when
the session ends.

## Socket and PID files

Both are stored under `XDG_RUNTIME_DIR` (or `/tmp` when unset):

| Project | Socket | PID file |
|---------|--------|----------|
| `/home/user/my-app` | `sat-hx-ide-{hash}.sock` | `sat-hx-ide-{hash}.pid` |

The hash is derived from the canonical project directory path. Each project gets
**its own socket and PID file** so concurrent sessions do not interfere.

Find your socket from the project directory:

```bash
# Same hash algorithm as src/ipc.rs
python3 -c "p=b'/home/user/my-app'; print(hex(sum((acc*31+b)%2**64 for acc,b in [(0,b) for b in p])))"
```

Or enable verbose logging and look for `IPC daemon listening on ...` at init time.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `SAT_HX_IDE_CONFIG` | Config path loaded by the daemon (set automatically at spawn) |
| `ZELLIJ_SESSION_NAME` | Session key for pane cache (set at init spawn) |
| `XDG_RUNTIME_DIR` | Base directory for sockets and PID files |

## Protocol

Line-delimited JSON over a Unix domain socket. See [`src/protocol.rs`](../src/protocol.rs).

**Requests:** `file_manager_open`, `terminal_toggle`, `git_open`, `ping`, `shutdown`, …

**Responses:** `ok`, `pong`, `error`, `pane_list`, `context`

## IPC limitations (Phase 1)

These actions **fall back to the process model** when the daemon cannot handle them:

- **File manager spawn** when no file-manager pane exists (needs `ZELLIJ_PANE_ID`)
- **File manager toggle-dock** and **run** (same constraint)

Terminal toggle/zoom and git open are handled fully via IPC when the daemon is running.

## Troubleshooting

### Actions feel slow

The helper process still starts on every keypress. IPC saves repeated config parsing
and `zellij list-panes` inside the daemon, not the Zellij→CLI spawn itself.

### Daemon not used

1. Check the socket exists: `ls $XDG_RUNTIME_DIR/sat-hx-ide-*.sock`
2. Run with `-v` / `RUST_LOG=debug` and look for `Falling back to process model`
3. Confirm `cwd` in keybindings matches the project root (set at init)

### Stale daemon after crash

Remove the socket and PID file for your project:

```bash
rm -f "$XDG_RUNTIME_DIR/sat-hx-ide-"*.sock "$XDG_RUNTIME_DIR/sat-hx-ide-"*.pid
```

Or restart the session with `sat-hx-ide init`.

### Two projects open at once

Each project must use its own socket/PID pair. If cleanup kills the wrong daemon,
verify PID files are named `sat-hx-ide-{hash}.pid`, not a shared `daemon.pid`.

## Further reading

- [`design.md`](design.md) — full architecture
- [`ipc-implementation-plan.md`](ipc-implementation-plan.md) — rollout plan and protocol spec
