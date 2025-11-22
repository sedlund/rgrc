# rgrc - Rusty Generic Colouriser

[![Rust](https://img.shields.io/badge/rust-2024--edition-orange)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, Rust-based command-line tool that colorizes the output of other commands using regex-based rules, similar to the classic `grc` (Generic Colouriser) utility.

## Features

- 🚀 **High Performance**: Written in Rust with parallel processing for large outputs
- 🎨 **Rich Colorization**: Supports ANSI colors, styles, and attributes
- 🔧 **Flexible Configuration**: Compatible with grc/grcat configuration files
- 🐚 **Shell Integration**: Generates aliases for popular commands
- 📖 **Comprehensive**: Supports 80+ pre-configured commands
- ⚡ **Adaptive**: Automatically chooses single-threaded or parallel processing

## Quick Start

### Installation

#### From Source
```bash
git clone https://github.com/lazywalker/rgrc.git
cd rgrc
make release
sudo make install
```

#### Using Cargo
```bash
cargo install rgrc
```

### Basic Usage

```bash
# Colorize ping output
rgrc ping -c 4 google.com

# Colorize ls output
rgrc ls -la

# Colorize any command
rgrc df -h
```

### Generate Shell Aliases

```bash
# Show aliases for supported commands
rgrc --aliases

# Generate aliases for all known commands
rgrc --all-aliases

# Add to your shell profile
echo 'eval "$(rgrc --aliases)"' >> ~/.bashrc
```

## Configuration

### Configuration Files

rgrc reads configuration from multiple locations:

```
~/.rgrc                    # User-specific config
~/.config/rgrc/rgrc.conf   # XDG config location
/usr/local/etc/rgrc.conf   # System-wide config
/etc/rgrc.conf            # Fallback system config
```

### grcat Compatibility

rgrc is fully compatible with grc/grcat configuration files:

```
~/.config/rgrc/           # rgrc-specific configs
~/.local/share/rgrc/      # User share directory
/usr/local/share/rgrc/    # Local share directory
/usr/share/rgrc/          # System share directory
~/.config/grc/            # grc compatibility
~/.local/share/grc/       # grc compatibility
/usr/local/share/grc/     # grc compatibility
/usr/share/grc/           # grc compatibility
```

### Supported Commands

rgrc comes with pre-configured colorization rules for 80+ commands:

**System Monitoring**: `df`, `free`, `ps`, `top`, `vmstat`, `iostat`
**Network Tools**: `ping`, `traceroute`, `netstat`, `ss`, `ip`, `curl`
**Development**: `gcc`, `make`, `docker`, `kubectl`, `git`
**File Operations**: `ls`, `find`, `du`, `mount`, `fdisk`
**And many more...**

## Command Line Options

```
Usage: rgrc [OPTIONS] COMMAND [ARGS...]

Options:
  --help              Show help message
  --aliases           Generate shell aliases for supported commands
  --all-aliases       Generate aliases for all known commands
  --except=CMD,...    Exclude commands from alias generation
  --color=on|off|auto Control color output (default: auto)
```

## Examples

### Basic Colorization
```bash
# Colorize ping output
rgrc ping -c 4 8.8.8.8

# Colorize disk usage
rgrc df -h

# Colorize compilation output
rgrc make

# Colorize Docker commands
rgrc docker ps
rgrc docker logs mycontainer
```

### Advanced Usage
```bash
# Force color output
rgrc --color=on ls -la

# Disable colors
rgrc --color=off ps aux

# Generate aliases for specific commands
rgrc --aliases | grep -E "(ping|ls|ps|docker)"

# Exclude certain commands from aliases
rgrc --all-aliases --except=docker,kubectl
```

### Shell Integration
```bash
# Bash
echo 'eval "$(rgrc --aliases)"' >> ~/.bashrc

# Zsh
echo 'eval "$(rgrc --aliases)"' >> ~/.zshrc

# Fish
rgrc --aliases > ~/.config/fish/conf.d/rgrc.fish
```

## Configuration Examples

### Custom Command Configuration

Create `~/.config/rgrc/conf.mycommand`:

```
regexp=^ERROR
colours=red,bold

regexp=^WARNING
colours=yellow

regexp=^INFO
colours=green
```

### Extending Existing Commands

Add rules to `~/.rgrc`:

```
# Custom rules for existing commands
regexp=^CUSTOM
colours=blue,underline
```

## Performance

- **Adaptive Processing**: Automatically chooses single-threaded (<1000 lines) or parallel processing
- **Zero-copy Operations**: Efficient memory usage with minimal allocations
- **Regex Optimization**: Uses fancy-regex for advanced pattern matching
- **ANSI Optimization**: Merges adjacent styles to reduce escape sequences

## Development

### Building from Source

```bash
# Clone repository
git clone https://github.com/lazywalker/rgrc.git
cd rgrc

# Build debug version
cargo build

# Build release version
cargo build --release

# Run tests
cargo test

# Generate documentation
cargo doc --open
```

### Project Structure

```
rgrc/
├── src/
│   ├── main.rs      # CLI entry point
│   ├── lib.rs       # Core library
│   ├── grc.rs       # Configuration parsing
│   └── colorizer.rs # Colorization engine
├── doc/
│   └── rgrc.1.md    # Manual page (markdown)
├── share/           # Pre-configured rules
├── etc/             # Shell completions
└── tests/           # Test files
```

### Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes and add tests
4. Run tests: `cargo test`
5. Submit a pull request

## Compatibility

- **Operating Systems**: Linux, macOS, Windows (with WSL)
- **Shells**: Bash, Zsh, Fish, and others
- **Terminals**: Any ANSI-compatible terminal
- **grc Compatibility**: Drop-in replacement for grc

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Credits

- Inspired by the original `grc` (Generic Colouriser) by Radovan Garabík
- Built with [Rust](https://www.rust-lang.org/) and [console](https://crates.io/crates/console)

## Related Projects

- [grc](https://github.com/garabik/grc) - Original Generic Colouriser (Python)
- [lolcat](https://github.com/busyloop/lolcat) - Rainbow coloring tool
- [bat](https://github.com/sharkdp/bat) - A cat clone with syntax highlighting
