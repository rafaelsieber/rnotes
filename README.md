# rnotes

A fast, terminal-based markdown notes manager with VIM-like keybindings and intuitive navigation.

## Features

✨ **VIM-inspired interface** - Familiar keybindings for efficient navigation  
📁 **Interactive file tree** - Browse and manage your markdown files  
📝 **Line-by-line navigation** - Navigate through file content with precision  
🖱️ **Mouse text selection** - Select and copy text using your mouse  
⚙️ **Configurable** - Set your preferred notes directory and editor  
🎨 **Syntax highlighting** - Color-coded file types in the tree view  
⚡ **Fast and lightweight** - Built with Rust for optimal performance  

## Installation

### From Source
```bash
git clone https://github.com/your-username/rnotes.git
cd rnotes
cargo build --release
./target/release/rnotes
```

## Usage

Launch rnotes from any directory:
```bash
rnotes
```

The application will start with your notes directory (defaults to `~/rnotes`).

### Key Bindings

#### File Tree Navigation
| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` / `→` | Expand folder / Enter line navigation |
| `i` | Edit selected file |
| `n` | Create new file |
| `d` | Create new folder |
| `r` | Rename file/folder |
| `x` | Delete file/folder (with confirmation) |
| `c` | Open configuration |
| `q` | Quit application |

#### Line Navigation Mode
When viewing a file, press `→` (Right arrow) to enter line navigation:

| Key | Action |
|-----|--------|
| `j` / `↓` | Move to next line |
| `k` / `↑` | Move to previous line |
| `y` | Copy current line to clipboard |
| `i` | Edit file |
| `←` / `Esc` | Return to file tree |

#### Configuration Mode
| Key | Action |
|-----|--------|
| `Tab` | Switch between fields |
| `Enter` | Save and exit |
| `Esc` | Cancel changes |

### Smart Navigation
- **Right arrow (→)** intelligently expands folders when on directories, or enters line navigation when on files
- **Mouse support** for text selection and copying
- **Contextual file operations** - create files/folders in the currently selected directory

## Configuration

Press `c` to configure:
- **Notes Directory**: Set your preferred notes location (default: `~/rnotes`)
- **Editor**: Choose your preferred text editor (default: system editor)

Configuration is automatically saved to your user config directory.

## File Types

The file tree uses color coding:
- 🟢 **Green**: Markdown files (`.md`)
- 🔵 **Cyan**: Directories
- ⚪ **Gray**: Other files

## Dependencies

- **ratatui**: Terminal UI framework
- **crossterm**: Cross-platform terminal manipulation
- **serde**: Configuration serialization
- **dirs**: User directory detection
- **anyhow**: Error handling
- **arboard**: Clipboard support

## Building

Requirements:
- Rust 1.70+
- Cargo

```bash
# Debug build
cargo build

# Release build (recommended)
cargo build --release

# Run directly
cargo run
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Roadmap

- [ ] Search functionality across files
- [ ] Markdown preview mode
- [ ] Custom themes and color schemes
- [ ] Plugin system
- [ ] Git integration
- [ ] Multiple workspace support
