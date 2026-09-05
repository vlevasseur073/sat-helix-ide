# Appendix

This appendix contains reference information, environment variables, related projects, and additional resources for `sat-helix-ide`.

## Command Reference

### Main Commands

| Command | Description | Usage |
|---------|-------------|-------|
| `init` | Initialize a new Zellij session | `sat-hx-ide init [PATH] [--session NAME] [--no-ai] [--no-attach]` |
| `doctor` | Check system requirements and configuration | `sat-hx-ide doctor` |
| `config` | Display current configuration | `sat-hx-ide config [--config PATH]` |
| `version` | Display version information | `sat-hx-ide version` |

### Helper Commands (Internal)

These commands are invoked internally by Zellij keybindings and are not typically called directly by users.

| Command | Description | Trigger |
|---------|-------------|---------|
| `__file-manager open` | Open or focus file manager | `Ctrl-y` |
| `__file-manager toggle-dock` | Toggle file manager between floating and docked | `Alt-y` |
| `__terminal toggle` | Hide/show terminal | `Alt-t` |
| `__terminal zoom` | Zoom terminal to full height or restore | `Alt-Shift-t` |
| `__git open` | Open Git TUI in floating pane | `Alt-g` |
| `__review open` | Open review tool in floating pane | `Alt-r` |
| `__workflow open` | Open workflow tool in floating pane | `Alt-w` |

### Command-Line Options

#### `init` Command Options

| Option | Description | Default |
|--------|-------------|---------|
| `[PATH]` | Project directory | Current directory |
| `--session NAME` | Session name override | Directory name |
| `--no-ai` | Skip AI tab | `false` |
| `--no-attach` | Don't attach to existing session | `false` |
| `--config PATH` | Custom configuration file | `~/.config/sat-helix-ide/config.toml` |

**Examples:**

```bash
# Initialize in current directory
sat-hx-ide init .

# Initialize in specific directory with custom session name
sat-hx-ide init ~/projects/myapp --session myapp-dev

# Initialize without AI tab
sat-hx-ide init . --no-ai

# Initialize with custom config
sat-hx-ide init . --config /path/to/config.toml
```

#### `config` Command Options

| Option | Description | Default |
|--------|-------------|---------|
| `--config PATH` | Custom configuration file | `~/.config/sat-helix-ide/config.toml` |

**Examples:**

```bash
# Show current configuration
sat-hx-ide config

# Show configuration from custom file
sat-hx-ide config --config /path/to/config.toml
```

## Environment Variables

### sat-helix-ide Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Logging level for sat-hx-ide | `info` |
| `XDG_RUNTIME_DIR` | Directory for runtime files | `/run/user/<uid>/` (Linux) |

### Zellij Variables

| Variable | Description | Effect |
|----------|-------------|--------|
| `ZELLIJ_CONFIG_FILE` | Path to Zellij configuration | Overrides `session.zellij_config` |
| `ZELLIJ_LOG` | Zellij logging level | Controls Zellij's own logging |

### Tool Variables

| Variable | Tool | Description |
|----------|------|-------------|
| `SHELL` | Terminal | Default shell for terminal panes |
| `HELIX_EDITOR` | Helix | Helix configuration directory (if needed) |
| `YAZI_CONFIG` | Yazi | Yazi configuration directory |

## File Locations

### Configuration Files

| Purpose | Location | Description |
|---------|----------|-------------|
| User Config | `~/.config/sat-helix-ide/config.toml` | Main configuration file |
| Sample Config | `<repo>/configs/config.toml` | Sample configuration for reference |
| Runtime Config | `$XDG_RUNTIME_DIR/sat-helix-ide/<session>/config.kdl` | Merged Zellij config |
| Runtime Layout | `$XDG_RUNTIME_DIR/sat-helix-ide/<session>/layout.kdl` | Session layout |

### Default Configuration

The default configuration is embedded in the binary. When no user configuration file exists, these defaults are used:

```toml
[session]
attach_existing = true
ai_by_default = true

[terminal]
enabled = true
dock_percent = 15
dock_position = "down"

[keybindings]
file_manager = "Ctrl y"
file_manager_dock = "Alt y"
git = "Alt g"
review = "Alt r"
workflow = "Alt w"
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
command = "lazygit"
args = []

[tools.review]
command = "revdiff"
args = []

[tools.workflow]
command = "glab-tui"
args = []

[tools.ai]
command = "cursor-agent"
args = []
```

## Keyboard Shortcuts Summary

### Default Keybindings

| Key | Action | Component |
|-----|--------|-----------|
| `Ctrl-y` | Open/focus file manager | File Manager |
| `Alt-y` | Toggle file manager dock | File Manager |
| `Alt-t` | Toggle terminal visibility | Terminal |
| `Alt-Shift-t` | Zoom terminal | Terminal |
| `Alt-g` | Open Git TUI | Git |
| `Alt-r` | Open review tool | Review |
| `Alt-w` | Open workflow tool | Workflow |

### Zellij Default Keybindings (Still Active)

| Key | Action | Mode |
|-----|--------|------|
| `Ctrl-q` | Quit Zellij | Normal |
| `Ctrl-g` | Toggle lock mode | All |
| `Ctrl-n` | New pane | Normal |
| `Ctrl-h/j/k/l` | Move focus | Normal |
| `Ctrl-Shift-h/j/k/l` | Move pane | Normal |
| `Ctrl-p` | Previous tab | Normal |
| `Ctrl-n` | Next tab | Normal |

## Configuration Reference

### All Configuration Options

#### `[session]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `attach_existing` | Boolean | `true` | Attach to existing session |
| `ai_by_default` | Boolean | `true` | Include AI tab |
| `zellij_config` | String | Auto | Custom Zellij config path |

#### `[terminal]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `enabled` | Boolean | `true` | Include terminal pane |
| `dock_percent` | Integer | `15` | Terminal size percentage |
| `dock_position` | String | `"down"` | Terminal position (`"down"` or `"right"`) |

#### `[keybindings]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `file_manager` | String | `"Ctrl y"` | File manager key |
| `file_manager_dock` | String | `"Alt y"` | File manager dock toggle |
| `git` | String | `"Alt g"` | Git TUI key |
| `review` | String | `"Alt r"` | Review tool key |
| `workflow` | String | `"Alt w"` | Workflow tool key |
| `terminal` | String | `"Alt t"` | Terminal toggle key |
| `terminal_zoom` | String | `"Alt Shift t"` | Terminal zoom key |

#### `[tools.zellij]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | String | `"zellij"` | Zellij executable |
| `args` | Array | `[]` | Zellij arguments |

#### `[tools.editor]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | String | `"hx"` | Editor executable |
| `args` | Array | `[]` | Editor arguments |

#### `[tools.file_manager]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `adapter` | String | `"yazi"` | File manager adapter |
| `command` | String | `"yazi"` | File manager executable |
| `args` | Array | `[]` | File manager arguments |
| `float_width` | String | `"100%"` | Floating width |
| `float_height` | String | `"100%"` | Floating height |
| `dock_percent` | Integer | `28` | Docked width percentage |

#### `[tools.git]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | String | `"lazygit"` | Git TUI executable |
| `args` | Array | `[]` | Git TUI arguments |

#### `[tools.review]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | String | `"revdiff"` | Review tool executable |
| `args` | Array | `[]` | Review tool arguments |

#### `[tools.workflow]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | String | `"glab-tui"` | Workflow tool executable |
| `args` | Array | `[]` | Workflow tool arguments |

#### `[tools.ai]`

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `command` | String | `"cursor-agent"` | AI agent executable |
| `args` | Array | `[]` | AI agent arguments |

## Related Projects

### Terminal Multiplexers

| Project | Description | Website |
|---------|-------------|---------|
| **Zellij** | Terminal workspace multiplexer (used by sat-hx-ide) | [zellij.dev](https://zellij.dev) |
| tmux | Terminal multiplexer | [tmux.github.io](https://tmux.github.io) |
| screen | Terminal multiplexer | [GNU Screen](https://www.gnu.org/software/screen/) |

### Editors

| Project | Description | Website |
|---------|-------------|---------|
| **Helix** | Modal text editor (used by sat-hx-ide) | [helix-editor.com](https://helix-editor.com) |
| Neovim | Vim-fork with modern features | [neovim.io](https://neovim.io) |
| Kakoune | Modal, multiple-selections editor | [kakoune.org](https://kakoune.org) |

### File Managers

| Project | Description | Website |
|---------|-------------|---------|
| **Yazi** | Terminal file manager (used by sat-hx-ide) | [yazi-rs.github.io](https://yazi-rs.github.io) |
| Ranger | Console file manager | [ranger.github.io](https://ranger.github.io) |
| lf | Terminal file manager | [github.com/gokcehan/lf](https://github.com/gokcehan/lf) |
| nnn | Fast terminal file manager | [github.com/jarun/nnn](https://github.com/jarun/nnn) |

### Git TUIs

| Project | Description | Website |
|---------|-------------|---------|
| **Lazygit** | Simple Git TUI (used by sat-hx-ide) | [github.com/jesseduffield/lazygit](https://github.com/jesseduffield/lazygit) |
| **GitUI** | Alternative Git TUI | [github.com/gitui-org/gitui](https://github.com/gitui-org/gitui) |
| Tig | Text-mode interface for Git | [jonas.github.io/tig](https://jonas.github.io/tig/) |
| Magit | Emacs Git interface | [magit.vc](https://magit.vc) |

### Review Tools

| Project | Description | Website |
|---------|-------------|---------|
| **revdiff** | Smart diff viewer (used by sat-hx-ide) | [github.com/umputun/revdiff](https://github.com/umputun/revdiff) |
| git-delta | Syntax-highlighting pager for git | [github.com/dandavison/delta](https://github.com/dandavison/delta) |
| Diff So Fancy | Good-lookin' diffs | [github.com/so-fancy/diff-so-fancy](https://github.com/so-fancy/diff-so-fancy) |

### Workflow Tools

| Project | Description | Website |
|---------|-------------|---------|
| **glab-tui** | GitLab/GitHub TUI (used by sat-hx-ide) | [github.com/rcieri/glab-tui](https://github.com/rcieri/glab-tui) |
| Gh CLI | GitHub CLI | [cli.github.com](https://cli.github.com) |
| Lab | GitLab CLI | [github.com/zaquestion/lab](https://github.com/zaquestion/lab) |

### AI Agents

| Project | Description | Website |
|---------|-------------|---------|
| Cursor Agent | AI code assistant | [cursor.com](https://cursor.com) |
| GitHub Copilot CLI | GitHub Copilot in CLI | [github.com/github/copilot-cli](https://github.com/github/copilot-cli) |

### Similar Projects

| Project | Description | Website |
|---------|-------------|---------|
| Zellij IDE | Zellij-based IDE | [github.com/mattsse/zellij-ide](https://github.com/mattsse/zellij-ide) |
| Tmux IDE | Tmux-based IDE configurations | Various |
| Neovim Remote | Neovim remote editing | Built-in |

## Resources

### Official Resources

- **Repository**: [github.com/vlevasseur073/sat-helix-ide](https://github.com/vlevasseur073/sat-helix-ide)
- **Issues**: [github.com/vlevasseur073/sat-helix-ide/issues](https://github.com/vlevasseur073/sat-helix-ide/issues)
- **Discussions**: [github.com/vlevasseur073/sat-helix-ide/discussions](https://github.com/vlevasseur073/sat-helix-ide/discussions)

### Zellij Resources

- **Documentation**: [zellij.dev/documentation](https://zellij.dev/documentation)
- **GitHub**: [github.com/zellij-org/zellij](https://github.com/zellij-org/zellij)
- **Discord**: Zellij Discord server

### Helix Resources

- **Documentation**: [helix-editor.com/docs](https://helix-editor.com/docs)
- **GitHub**: [github.com/helix-editor/helix](https://github.com/helix-editor/helix)
- **Matrix**: Helix Matrix channel

### Yazi Resources

- **Documentation**: [yazi-rs.github.io/docs](https://yazi-rs.github.io/docs)
- **GitHub**: [github.com/sxyazi/yazi](https://github.com/sxyazi/yazi)

### Lazygit Resources

- **Documentation**: [github.com/jesseduffield/lazygit](https://github.com/jesseduffield/lazygit)
- **Config Guide**: [github.com/jesseduffield/lazygit/blob/master/docs/Config.md](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md)

## Performance Tips

### Optimizing Startup Time

1. **Pre-load Zellij**: Keep a Zellij session running
2. **Use SSD**: Faster disk I/O for configuration loading
3. **Reduce config complexity**: Simplify your Zellij configuration
4. **Cache runtime files**: Runtime files are cached across sessions

### Optimizing Runtime Performance

1. **Use pane cache**: The built-in pane cache reduces Zellij queries
2. **Avoid frequent resizes**: Resize operations bypass the cache
3. **Use efficient shells**: Some shells start faster than others
4. **Limit AI tab**: Disable AI tab if not needed (`--no-ai`)

### Memory Usage

- **No daemon**: No persistent memory usage
- **Short-lived processes**: Minimal memory footprint per action
- **Cleanup**: Runtime files are automatically cleaned up

## Integration Examples

### Integrating with Shell Aliases

```bash
# Add to your .bashrc or .zshrc
alias ide="sat-hx-ide init ."
alias ide-no-ai="sat-hx-ide init . --no-ai"

# Usage
ide        # Start session in current directory
ide-no-ai  # Start session without AI tab
```

### Integrating with Project-Specific Configs

```bash
# In your project's .env file or shell profile
if [ -d ".sat-hx-ide" ]; then
    export SAT_HX_IDE_CONFIG=".sat-hx-ide/config.toml"
fi

# Then use
sat-hx-ide init . --config $SAT_HX_IDE_CONFIG
```

### Integrating with Makefile

```makefile
.PHONY: ide
ide:
	sat-hx-ide init .

.PHONY: ide-debug
ide-debug:
	RUST_LOG=debug sat-hx-ide init .
```

### Integrating with Justfile

```just
# Start IDE
ide:
    sat-hx-ide init .

# Start IDE with debugging
ide-debug:
    RUST_LOG=debug sat-hx-ide init .

# Run doctor
check:
    sat-hx-ide doctor
```

## Version History

| Version | Date | Description |
|---------|------|-------------|
| 0.1.0 | Current | Initial release |

*Check the GitHub repository for the latest version and changelog.*

## License

`sat-helix-ide` is licensed under the MIT License. See the [LICENSE](https://github.com/vlevasseur073/sat-helix-ide/blob/main/LICENSE) file for details.

## Contributing

See [Development](./development.md) for information on contributing to `sat-helix-ide`.

## Support

- **GitHub Issues**: [github.com/vlevasseur073/sat-helix-ide/issues](https://github.com/vlevasseur073/sat-helix-ide/issues)
- **Discussions**: [github.com/vlevasseur073/sat-helix-ide/discussions](https://github.com/vlevasseur073/sat-helix-ide/discussions)
- **Contributing**: Pull requests welcome!

---

This concludes the `sat-helix-ide` User Manual & Design Document. We hope this comprehensive guide helps you get the most out of this powerful development tool.

- [Back to Introduction](./introduction.md) - Start over
- [Quick Start](./quick-start.md) - Get started quickly
- [User Manual](./user-manual.md) - Learn how to use
- [Configuration](./configuration.md) - Customize your setup
- [Architecture](./architecture.md) - Understand the design
