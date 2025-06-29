# rnotes

A fast, terminal-based markdown notes manager with VIM-like keybindings and intWhen Git integration is enabled:
- 🔄 **Auto-sync on startup**: Notes are automatically synchronized with remote repository when you launch RNotes
- 📤 **Manual push**: Press `g` to commit current changes and push to remote repository
- 📥 **Manual pull**: Press `p` to pull changes from remote repository
- 📊 **Status display**: Git status is shown in the top bar
- 🙈 **Clean interface**: Hidden files and `.git` directory are automatically filtered from view
- 🔧 **Easy setup**: Configure your repository URL, username, and email in the configuration screen navigation.

## Features

✨ **VIM-inspired interface** - Familiar keybindings for efficient navigation  
📁 **Interactive file tree** - Browse and manage your markdown files  
📝 **Line-by-line navigation** - Navigate through file content with precision  
🖱️ **Mouse text selection** - Select and copy text using your mouse  
⚙️ **Configurable** - Set your preferred notes directory and editor  
🎨 **Syntax highlighting** - Color-coded file types in the tree view  
⚡ **Fast and lightweight** - Built with Rust for optimal performance  
🔄 **Git integration** - Automatic commits and sync with remote repositories  

## Installation

### From Source
```bash
git clone https://github.com/yourusername/rnotes.git
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
| `g` | Git push (commit and push changes) |
| `p` | Git pull (pull changes from remote) |
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
- **Git Integration**: Enable/disable Git support
- **Git Repository**: URL of your Git repository
- **Git Username**: Your Git username for commits
- **Git Email**: Your Git email for commits

Configuration is automatically saved to your user config directory.

### Git Integration

When Git integration is enabled:
- 🔄 **Manual push**: Press `g` to commit current changes and push to remote repository
- � **Manual pull**: Press `p` to pull changes from remote repository
- 📊 **Status display**: Git status is shown in the top bar
- 🙈 **Clean interface**: Hidden files and `.git` directory are automatically filtered from view
- 🔧 **Easy setup**: Configure your repository URL, username, and email in the configuration screen

#### Prerequisites for Git Integration

1. **Install GitHub CLI**: `sudo apt install gh` (Ubuntu/Debian) or `brew install gh` (macOS)
2. **Authenticate**: Run `gh auth login` and follow the prompts
3. **Verify**: Check with `gh auth status` to ensure you're logged in

#### Setting up Git Integration

1. Press `c` to open configuration
2. Navigate to "Git Enabled" and press any key to toggle it on
3. Set your "Git Repository URL" (e.g., `https://github.com/yourusername/your-notes.git`)
4. Set your "Git Username" and "Git Email"
5. Press `Enter` to save

RNotes will automatically sync with your remote repository when you start the application. Your changes will stay local until you manually push them with `g`, and you can pull remote changes with `p`.

📖 **For detailed setup instructions, see [GIT_SETUP.md](GIT_SETUP.md)**

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
- **git2**: Git integration
- **chrono**: Date and time handling

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
- [x] Git integration
- [ ] Multiple workspace support
