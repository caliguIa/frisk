# frisk

A fast application launcher and search tool for macOS. Written in Rust with native Cocoa bindings.

## Usage

Launcher with no sources:

```bash
frisk
```

Include specific sources:

```bash
frisk --apps                    # Applications from /Applications
frisk --homebrew                # Homebrew packages
frisk --clipboard               # Clipboard history
frisk --commands                # Custom commands from config
frisk --nixpkgs                 # NixOS packages
frisk --dictionary              # Dictionary definitions
```

Combine multiple sources:

```bash
frisk --apps --homebrew --commands
```

Load custom sources:

```bash
frisk --source /path/to/custom.bin
```

Override config or prompt:

```bash
frisk --config /path/to/config.toml
frisk --prompt "Search: "
```

## Background Services

Some sources require background daemons to collect and cache data.

### Installing Services

```bash
# Install LaunchAgent plist files
frisk service install apps
frisk service install homebrew
frisk service install clipboard
frisk service install nixpkgs

# Start services
frisk service start apps
frisk service start homebrew
frisk service start clipboard
frisk service start nixpkgs

# Check status
frisk service status apps

# Stop and uninstall
frisk service stop apps
frisk service uninstall apps
```

Services write binary cache files to `$XDG_CACHE_HOME/frisk/`:
- `apps.bin` - Applications discovered from `/Applications` and a few other directories
- `homebrew.bin` - Homebrew formulae and casks
- `clipboard.bin` - Recent clipboard entries
- `nixpkgs.bin` - nixpkgs packages

### Manual Daemon Usage

You can also run daemons directly (useful for testing):

```bash
frisk daemon apps
frisk daemon homebrew
frisk daemon clipboard
frisk daemon dictionary
```

## Features

### Calculator

Type math expressions directly:

```
2 + 2
```

Press Enter to copy the result.

### Custom Commands

Define custom commands in `~/.config/frisk/commands.toml`:

```toml
[[command]]
name = "Hombrew search"
command = "frisk --homebrew"

[[command]]
name = "Clipboard history"
command = "frisk --clipboard"

[[command]]
name = "Empty Trash"
action = "osascript -e 'tell application \"Finder\" to empty trash'"

[[command]]
name = "Restart"
action = "osascript -e 'tell application \"System Events\" to restart'"
```

### Configuration

Configuration file: `~/.config/frisk/config.toml`

```toml
prompt = "Run: "
font_family = "Berkeley Mono"
font_size = 32.0

[styles]
background = "#282c34f0"
items = "#ffffff"
selected_item = "#61afef"
prompt = "#98c379"
query = "#e06c75"
caret = "#e06c75"

[spacing]
window_padding_x = 20.0
window_padding_y = 20.0
prompt_to_items = 60.0
item_spacing = 15.0
```

The config file is created automatically with defaults on first run.

