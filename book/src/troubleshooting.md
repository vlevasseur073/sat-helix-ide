# Troubleshooting

This chapter covers common issues and their solutions when using `sat-helix-ide`.

## Common Issues

### Installation Issues

#### "Command not found" after installation

**Problem**: After running `cargo install --path .`, the `sat-hx-ide` command is not found.

**Solutions**:

1. **Check Cargo's bin directory**:
   ```bash
   # Find where Cargo installs binaries
   echo $CARGO_HOME/bin

   # Add to PATH if not already there
   export PATH="$CARGO_HOME/bin:$PATH"
   ```

2. **Verify installation**:
   ```bash
   # Check if binary exists
   ls ~/.cargo/bin/sat-hx-ide

   # If it exists, add cargo bin to your shell's PATH
   echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
   source ~/.bashrc
   ```

3. **Reinstall**:
   ```bash
   cargo install --path . --force
   ```

#### Missing Required Tools

**Problem**: `sat-hx-ide doctor` reports missing required tools.

**Solution**: Install the missing tools:

- **Zellij**: Follow [Zellij installation](https://zellij.dev/documentation/installation.html)
- **Helix**: Follow [Helix installation](https://helix-editor.com/)
- **Yazi**: Follow [Yazi installation](https://yazi-rs.github.io/docs/installation)

Verify each tool is in your PATH:
```bash
which zellij
which hx
which yazi
```

### Session Issues

#### Session fails to start

**Problem**: `sat-hx-ide init .` fails with an error.

**Common causes and solutions**:

1. **Keybinding conflicts**:
   ```bash
   # Check for conflicts
   RUST_LOG=debug sat-hx-ide init .

   # Solution: Change conflicting keybindings in your config
   hx ~/.config/sat-helix-ide/config.toml
   ```

2. **Missing tools**:
   ```bash
   # Run doctor to check
   sat-hx-ide doctor

   # Install any missing tools
   ```

3. **Permission issues**:
   ```bash
   # Check runtime directory permissions
   ls -la $XDG_RUNTIME_DIR/sat-helix-ide/

   # Create runtime directory if missing
   mkdir -p $XDG_RUNTIME_DIR/sat-helix-ide/
   ```

#### Session starts but tools don't work

**Problem**: The Zellij session starts, but keybindings don't work or tools don't open.

**Solutions**:

1. **Check Zellij logs**:
   ```bash
   ZELLIJ_LOG=debug zellij attach <session>
   ```

2. **Verify keybindings**:
   - Ensure you're not in locked mode (press `Ctrl-g` to unlock)
   - Check that you're in normal mode
   - Verify keybindings are loaded:
     ```bash
     # Look for sat-hx-ide bindings in your runtime config
     cat $XDG_RUNTIME_DIR/sat-helix-ide/<session>/config.kdl | grep "sat-hx-ide"
     ```

3. **Restart session**:
   ```bash
   # Quit and restart
   zellij kill-session <session>
   sat-hx-ide init .
   ```

### Keybinding Issues

#### Keybindings don't work

**Problem**: Pressing the configured keys doesn't trigger any action.

**Solutions**:

1. **Check Zellij mode**:
   - Keybindings only work in normal mode, not locked mode
   - Press `Ctrl-g` to toggle lock mode
   - Look for mode indicators in Zellij's status bar

2. **Test keybindings manually**:
   ```bash
   # Run helper commands directly to test
   sat-hx-ide __file-manager open
   sat-hx-ide __terminal toggle
   ```

3. **Check for conflicts**:
   ```bash
   # List all keybindings in your Zellij config
   zellij --config /dev/null run -c 'list-keybindings'
   ```

#### Custom keybindings not working

**Problem**: Custom keybindings in configuration are not working.

**Solutions**:

1. **Validate keybinding syntax**:
   ```toml
   # Good
   file_manager = "Ctrl y"

   # Bad (extra spaces)
   file_manager = " Ctrl y "
   ```

2. **Check for conflicts**:
   ```bash
   # Test with debug logging
   RUST_LOG=debug sat-hx-ide init .

   # Look for conflict messages
   ```

3. **Try a different keybinding**:
   ```toml
   [keybindings]
   file_manager = "Ctrl f"  # Try a different key
   ```

### Terminal Issues

#### Terminal doesn't appear

**Problem**: The terminal pane is missing from the layout.

**Solutions**:

1. **Check configuration**:
   ```toml
   [terminal]
   enabled = true  # Make sure this is true
   ```

2. **Verify terminal command**:
   ```bash
   # Check if your shell is valid
   which $SHELL

   # Test your shell directly
   $SHELL --version
   ```

3. **Try with explicit terminal settings**:
   ```toml
   [terminal]
   enabled = true
   dock_percent = 20
   dock_position = "down"
   ```

#### Terminal exits immediately

**Problem**: The terminal appears but exits immediately.

**Solutions**:

1. **Check shell configuration**:
   - Some shells (like `fish`) may exit if they detect they're not interactive
   - Try with `bash` or `zsh`:
     ```bash
     # Test with bash
     ZELLIJ_SHELL=/bin/bash sat-hx-ide init .
     ```

2. **Check for errors in shell startup files**:
   ```bash
   # Test your shell startup files
   bash --login -c "echo Shell started successfully"
   ```

### File Manager Issues

#### Yazi doesn't open

**Problem**: Pressing `Ctrl-y` doesn't open Yazi.

**Solutions**:

1. **Check Yazi installation**:
   ```bash
   which yazi
   yazi --version
   ```

2. **Test Yazi directly**:
   ```bash
   # Run Yazi manually
   yazi
   ```

3. **Check file manager configuration**:
   ```toml
   [tools.file_manager]
   adapter = "yazi"
   command = "yazi"
   args = []
   ```

#### Files don't open in Helix

**Problem**: Selecting files in Yazi doesn't open them in Helix.

**Solutions**:

1. **Check Helix installation**:
   ```bash
   which hx
   hx --version
   ```

2. **Test file opening manually**:
   ```bash
   # Try opening a file with Helix
   hx /path/to/file.rs
   ```

3. **Check if Helix pane exists**:
   - The file should open in the existing named Helix pane
   - Make sure you haven't closed the Helix pane

### Git TUI Issues

#### Lazygit/GitUI doesn't open

**Problem**: Pressing `Alt-g` doesn't open the Git TUI.

**Solutions**:

1. **Check Git TUI installation**:
   ```bash
   which lazygit  # or gitui
   lazygit --version
   ```

2. **Test Git TUI directly**:
   ```bash
   # Run in the project directory
   cd /your/project
   lazygit
   ```

3. **Check configuration**:
   ```toml
   [tools.git]
   command = "lazygit"  # or "gitui"
   args = []
   ```

#### Git TUI can't find Git repository

**Problem**: The Git TUI opens but reports no repository found.

**Solutions**:

1. **Check current directory**:
   - The Git TUI runs in the current working directory of the Zellij session
   - Make sure you're in a Git repository

2. **Verify Git repository**:
   ```bash
   git status
   ```

3. **Restart session in correct directory**:
   ```bash
   cd /path/to/your/git/repo
   sat-hx-ide init .
   ```

### Configuration Issues

#### Configuration file not found

**Problem**: `sat-hx-ide` reports that the configuration file is not found.

**Solutions**:

1. **Check configuration locations**:
   ```bash
   # Default location
   ls ~/.config/sat-helix-ide/config.toml

   # Custom location
   ls /path/to/your/config.toml
   ```

2. **Specify configuration file**:
   ```bash
   sat-hx-ide init . --config /path/to/config.toml
   ```

3. **Create default configuration**:
   ```bash
   mkdir -p ~/.config/sat-helix-ide
   cp /path/to/sat-helix-ide/configs/config.toml ~/.config/sat-helix-ide/
   ```

#### Configuration validation errors

**Problem**: `sat-hx-ide` reports configuration validation errors.

**Solutions**:

1. **Check for syntax errors**:
   ```bash
   # Validate TOML syntax
   python3 -c "import tomllib; tomllib.loads(open('.config/sat-helix-ide/config.toml', 'rb').read())"
   ```

2. **Check for invalid values**:
   - `dock_percent` must be between 1 and 99
   - `dock_position` must be "down" or "right"
   - Command paths must exist

3. **Use defaults**:
   ```bash
   # Temporarily use bundled defaults
   mv ~/.config/sat-helix-ide/config.toml ~/.config/sat-helix-ide/config.toml.bak
   sat-hx-ide init .
   ```

## Debug Mode

### Enabling Debug Logging

Use the `RUST_LOG` environment variable to enable debug logging:

```bash
# Basic debug logging
RUST_LOG=debug sat-hx-ide init .

# Detailed trace logging
RUST_LOG=trace sat-hx-ide init .

# Only show info from specific modules
RUST_LOG=debug,sat_hx_ide::actions=trace sat-hx-ide init .
```

### Debug Log Levels

| Level | Description | Use Case |
|-------|-------------|----------|
| `error` | Only errors | Production troubleshooting |
| `warn` | Warnings and errors | General troubleshooting |
| `info` | Informational messages | Understanding flow |
| `debug` | Detailed debug info | Deep troubleshooting |
| `trace` | Very verbose, all details | Development debugging |

### Debugging Specific Operations

#### Debug Session Initialization

```bash
RUST_LOG=debug sat-hx-ide init . --session debug-session
```

This shows:
- Configuration file loading
- Layout generation
- Zellij session creation
- Keybinding injection

#### Debug Action Execution

```bash
# Start a session
sat-hx-ide init .

# In another terminal, debug a specific action
RUST_LOG=trace sat-hx-ide __file-manager open
```

#### Debug Zellij Integration

```bash
# Enable Zellij logging
ZELLIJ_LOG=debug zellij attach <session>

# Or set in environment
export ZELLIJ_LOG=debug
zellij attach <session>
```

## Environment-Specific Issues

### Running on Different Platforms

#### Linux

**Issue**: Runtime directory permissions.

**Solution**:
```bash
# Create runtime directory with proper permissions
mkdir -p ~/.local/share/sat-helix-ide
chmod 700 ~/.local/share/sat-helix-ide
```

#### macOS

**Issue**: No `XDG_RUNTIME_DIR` on macOS.

**Solution**:
```bash
# Set XDG_RUNTIME_DIR for macOS
export XDG_RUNTIME_DIR=/tmp
sat-hx-ide init .
```

#### Windows (WSL)

**Issue**: Windows paths don't work with Linux tools.

**Solution**:
```bash
# Use WSL paths
[tools.editor]
command = "hx.exe"
```

### Terminal Emulator Issues

#### Colors don't display correctly

**Problem**: Terminal colors are wrong or missing.

**Solutions**:

1. **Check terminal color support**:
   ```bash
   # Test color support
   echo -e "\033[31mRed\033[0m"
   ```

2. **Set proper TERM variable**:
   ```bash
   export TERM=xterm-256color
   ```

3. **Check Zellij color configuration**:
   - Ensure your Zellij config has proper color settings
   - Check Zellij's theme configuration

#### Font rendering issues

**Problem**: Characters don't display correctly.

**Solutions**:

1. **Use a proper font**:
   - Nerd Fonts (recommended for icons)
   - Fira Code
   - JetBrains Mono

2. **Check terminal font settings**:
   - Ensure your terminal is using a monospace font
   - Set font size appropriately

## Known Limitations

### Zellij Version Compatibility

**Issue**: Different Zellij versions may have different features or behaviors.

**Solution**:
- Use the latest stable version of Zellij
- Check Zellij's changelog for breaking changes
- Report compatibility issues to the sat-helix-ide repository

### Tool Version Compatibility

**Issue**: Different versions of integrated tools (Helix, Yazi, Lazygit) may behave differently.

**Solution**:
- Use reasonably recent versions of all tools
- Check tool-specific documentation for version requirements
- Report compatibility issues

### Wayland Support

**Issue**: Some features may not work perfectly under Wayland.

**Solution**:
- Most functionality should work under Wayland
- Some clipboard or focus issues may occur
- Try running under X11 if you encounter issues

## Getting Help

### Checking System Information

To diagnose issues, gather the following information:

```bash
# System info
echo "OS: $(uname -a)"
echo "Shell: $SHELL"
echo "Zellij: $(zellij --version 2>&1)"
echo "Helix: $(hx --version 2>&1)"
echo "Yazi: $(yazi --version 2>&1)"
echo "Lazygit: $(lazygit --version 2>&1 || echo 'Not installed')"
echo "sat-hx-ide: $(sat-hx-ide version 2>&1)"

# Configuration
cat ~/.config/sat-helix-ide/config.toml 2>/dev/null || echo "No custom config"

# Environment
env | grep -E "(XDG|TERM|PATH|ZELLIJ)" | head -10
```

### Reporting Issues

When reporting issues on GitHub, include:

1. **sat-hx-ide version**: `sat-hx-ide version`
2. **Tool versions**: Zellij, Helix, Yazi, Git TUI
3. **OS and platform**: Linux/macOS/Windows, distribution
4. **Configuration**: Your `config.toml` (if custom)
5. **Steps to reproduce**: What you did to trigger the issue
6. **Expected vs actual behavior**: What should happen vs what happened
7. **Debug logs**: Output with `RUST_LOG=debug` if applicable

### Creating a Minimal Reproduction

To help diagnose issues, create a minimal reproduction:

1. Start with a fresh configuration
2. Use only the default settings
3. Reproduce the issue with minimal steps
4. Document the exact steps to reproduce

## Troubleshooting Checklist

- [ ] Run `sat-hx-ide doctor` for system check
- [ ] Check `RUST_LOG=debug sat-hx-ide init .` for debug output
- [ ] Verify all required tools are installed and in PATH
- [ ] Test each tool individually (zellij, hx, yazi, lazygit)
- [ ] Check configuration file syntax and values
- [ ] Try with default configuration (no custom config)
- [ ] Test in a fresh directory/session
- [ ] Check for keybinding conflicts
- [ ] Verify terminal emulator compatibility
- [ ] Search existing issues on GitHub

## Next Steps

- [Appendix](./appendix.md) - Command reference and additional resources
- [GitHub Issues](https://github.com/vlevasseur073/sat-helix-ide/issues) - Report or browse issues
- [Discussions](https://github.com/vlevasseur073/sat-helix-ide/discussions) - Ask questions and share ideas
