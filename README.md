# sat-helix-ide
A workspace orchestrator for a full IDE based on Helix + Zellij + Yazi + lazygit/gitui + git-delta
It launches [Helix](https://helix-editor.com/), [Zellij](https://zellij.dev/), [Yazi](https://yazi-rs.github.io/), and [Lazygit](https://github.com/jesseduffield/lazygit) or [GitUI](https://github.com/gitui-org/gitui) as one terminal IDE.

**Motto: sat-helix-ide never overwrites your existing tool configuration.** Zellij, Helix, Yazi, Lazygit, GitUI, and global Git settings stay untouched. Optional overlays live only under sat-helix-ide's own directories and are used for a session when you opt in.

## Requirements

Install the tools you want in the layout:

| Tool | Required |
| --- | --- |
| [Zellij](https://zellij.dev/) | yes |
| [Helix](https://helix-editor.com/) (`hx`) | yes |
| [Yazi](https://yazi-rs.github.io/) | optional (file explorer panes) |
| [Lazygit](https://github.com/jesseduffield/lazygit) or [GitUI](https://github.com/gitui-org/gitui) | optional (git panes) |
| [git-delta](https://github.com/dandavison/delta) | optional (diff pager check only) |

Rust 1.80+ is enough to build from source.

## Install

```bash
cargo install --path .
```

The binary is named `sat-hx-ide`. After `cargo install --path .` that is the command to use.

## Usage

```bash
sat-hx-ide doctor
sat-hx-ide list-layouts
sat-hx-ide init .
sat-hx-ide init /path/to/project --layout coding --new-session
```

| Command | Purpose |
| --- | --- |
| `init` / `start` | Open a Zellij session with a layout in the given project |
| `list-layouts` / `ls` | List layouts from the sat-helix-ide config |
| `config` | Print the loaded configuration |
| `generate` / `gen` | Write optional overlays under sat-helix-ide's private config dir |
| `doctor` / `check` | Check that required tools are on `PATH` |
| `version` | Print version |

`--layout` defaults to `default_layout` in the config (`default`). `--new-session` forces `zellij --new-session-with-layout`; otherwise the layout is opened with `zellij --layout`.

## Configuration

sat-helix-ide looks at `~/.config/sat-helix-ide/config.toml` (or `$XDG_CONFIG_HOME/sat-helix-ide/config.toml`). If that file is missing, bundled defaults from `configs/config.toml` are used **in memory**. The missing file is not created.

Last workspace is stored separately in `$XDG_STATE_HOME/sat-helix-ide/state.toml` (typically `~/.local/state/sat-helix-ide/state.toml`). That write never targets Zellij, Helix, Yazi, or Git client configs.

Copy the sample to start customizing:

```bash
mkdir -p ~/.config/sat-helix-ide
cp configs/config.toml ~/.config/sat-helix-ide/config.toml
```

### Tool paths and shell aliases

Layout panes are launched by Zellij directly, not through your shell, so **shell aliases and functions are not visible to them**. A pane command such as `hx` is mapped onto the matching `[tools]` entry and then resolved on `PATH`:

| Layout command | Config key |
| --- | --- |
| `hx`, `helix` | `tools.helix.path` |
| `yazi` | `tools.yazi.path` |
| `lazygit`, `gitui` | `tools.git.client` |

If a tool is not on `PATH` (for example Helix installed via snap, where `hx` is only a shell alias), set an absolute path:

```toml
[tools.helix]
path = "/snap/bin/hx"
```

`init` resolves every command before starting Zellij and fails with the unresolved name, rather than opening a broken pane.

### Layouts

Bundled layouts (from `configs/config.toml`):

| Name | Contents |
| --- | --- |
| `default` | Helix, Yazi, Lazygit |
| `coding` | Helix and Yazi |
| `review` | Helix and Lazygit |
| `minimal` | Helix and Yazi side by side |

`layouts/*.kdl` are reference Zellij files. Runtime layouts come from the TOML config and are converted to current Zellij KDL.

### Optional generated overlays

By default `use_generated_configs = false`, so Zellij, Helix, and Yazi keep using **your** configs.

To experiment with sat-helix-ide overlays without touching `~/.config/{zellij,helix,yazi}`:

```bash
sat-hx-ide generate --all
```

That writes only under `$XDG_CONFIG_HOME/sat-helix-ide/generated/`. Enable them in the sat-helix-ide config:

```toml
use_generated_configs = true
```

Then `init` passes `--config` to Zellij, `YAZI_CONFIG_HOME` to Yazi, and `--config` to Helix panes. If those generated files are missing, `init` fails rather than falling back to writing into your tool directories.

`generate` never writes `git config --global`. The generated Zellij overlay only sets `theme` and `mouse_mode`; it does **not** add keybinds.

## Scripts

`scripts/` is leftover from the draft and is **not used** by `sat-hx-ide`.

| Script | Purpose | Useful? |
| --- | --- | --- |
| `scripts/helix-ide` | Thin wrapper: `exec sat-hx-ide` if installed, else `cargo run`. | Optional. After install, call `sat-hx-ide` directly. Handy only while iterating from a source checkout. |
| `scripts/open-in-helix` | Intended Yazi opener that finds a Helix pane and opens the selected file there. | **No.** Nothing calls it. It uses Zellij CLI that 0.44 does not provide (`zellij query --panes`, `zellij action --pane-id … run`) and depends on `jq`. Opening a file from Yazi uses Yazi's own opener (`hx`, or `$EDITOR`). |

Leave them in the tree if you want a local alias, but they are not part of the orchestrator.

## Navigating panes

sat-hx-ide does not install or override Zellij, Helix, Yazi, or Git-client keybinds. After `init`, you are in a Zellij session using **your existing tool configs** (`use_generated_configs = false` by default).

In the default layout, Helix is the large left pane; Yazi is top-right; Lazygit is bottom-right. Click a pane if Zellij `mouse_mode` is on (it is by default).

Zellij's stock bindings (and the usual customized dump of them) move focus **without** a prefix, which is what you want while Helix has the keyboard:

| Keys | Action |
| --- | --- |
| `Alt`+`h` / `j` / `k` / `l` | Focus left / down / up / right (`h`/`l` also switch tab at the edge) |
| `Alt`+arrows | Same as `Alt`+`hjkl` |
| `Ctrl`+`p`, then `h`/`j`/`k`/`l` | Pane mode, then move focus |
| `Ctrl`+`p`, then `p` | Cycle focus |
| `Ctrl`+`p`, then `f` | Toggle fullscreen on the focused pane |
| `Ctrl`+`p`, then `n` / `d` / `r` | New pane (auto / down / right) |
| `Ctrl`+`p`, then `x` | Close focused pane |
| `Ctrl`+`n`, then `hjkl` or `+/-` | Resize mode |
| `Ctrl`+`t`, then `n` / `x` / `1`–`9` | Tabs |
| `Ctrl`+`o`, then `w` | Session manager |
| `Ctrl`+`g` | Toggle locked mode (Zellij ignores other prefixes until `Ctrl`+`g` again) |
| `Ctrl`+`q` | Quit Zellij |

`Alt`+`hjkl` is the practical way to leave Helix for Yazi or Lazygit. Helix, Yazi, and Lazygit keep their own keys while focused; they do not see Zellij's `Ctrl`+`p` mode until you send those keys to Zellij.

Your own Helix config is unchanged. Typical extra bindings people already have (not provided by sat-hx-ide) include Helix `Space`+`g` for floating git tools and `Ctrl`+`y` for a Yazi chooser; those continue to work because sat-hx-ide does not replace `~/.config/helix/config.toml`.

If a prefix seems dead, check that the pane is not in Zellij locked mode (`Ctrl`+`g`) and that the focused app is not eating the key (Helix insert mode, Lazygit, Yazi).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Install [pre-commit](https://pre-commit.com/) and enable the hooks:

```bash
pre-commit install
pre-commit run --all-files
```

GitHub Actions on `push` and `pull_request` run the same formatter, Clippy, and test checks.
