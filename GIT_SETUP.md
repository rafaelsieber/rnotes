# Git Integration Setup Guide

This guide explains how to set up Git integration in RNotes with your private repository.

## Quick Setup

1. **Enable Git Integration**
   - Launch RNotes: `./target/release/rnotes`
   - Press `c` to open configuration
   - Navigate to "Git Enabled" and press any key to toggle it to `true`

2. **Configure Repository**
   - Set "Git Repository URL" to: `https://github.com/yourusername/your-notes-repo.git`
   - Set "Git Username" to your GitHub username
   - Set "Git Email" to your GitHub email
   - Press `Enter` to save configuration

3. **Prerequisites**
   - Install GitHub CLI: `sudo apt install gh` (Ubuntu/Debian) or `brew install gh` (macOS)
   - Authenticate with GitHub: `gh auth login`
   - Verify authentication: `gh auth status`

## How It Works

### Automatic Sync
- **Auto-pull on startup**: When you launch RNotes, it automatically syncs with your remote repository
- **Manual Push**: Press `g` to commit all current changes and push to remote repository
- **Manual Pull**: Press `p` to pull changes from remote repository
- **No Auto-commits**: Changes are NOT automatically committed - you have full control

### Commit Messages
When you push with `g`, all commits use generic messages with timestamps:
- `"Manual commit from RNotes - 2025-06-29 14:30:00 UTC"`

### Clean Interface
- Hidden files (starting with `.`) are filtered from the file tree
- The `.git` directory is not visible in the interface
- Only markdown files and regular directories are shown

## Repository Structure

Your notes repository will look like this:
```
your-notes-repo/
├── .gitignore          # Created automatically
├── welcome.md          # Default welcome file
├── note_1234567890.md  # Your notes (timestamped)
└── folder_1234567891/  # Folders you create
    └── more_notes.md
```

## Troubleshooting

### Authentication Issues
If you get authentication errors:
1. Make sure GitHub CLI is installed and configured: `gh auth status`
2. Re-authenticate if needed: `gh auth login`
3. Use HTTPS URLs (recommended): `https://github.com/yourusername/your-notes-repo.git`

### GitHub CLI Setup
If you don't have GitHub CLI set up:
```bash
# Install GitHub CLI (Ubuntu/Debian)
sudo apt install gh

# Or on macOS
brew install gh

# Authenticate
gh auth login

# Verify authentication
gh auth status
```

### First Push
If this is a new repository, you might need to push manually the first time:
```bash
cd ~/rnotes  # or your configured notes directory
git remote add origin https://github.com/yourusername/your-notes-repo.git
git branch -M main
git push -u origin main
```

### Git Status Indicators
- `Git: ✓` - No changes, repository is clean
- `Git: 3 changes` - There are 3 modified/untracked files
- `Git: ⚠` - Git error (check configuration)

### Key Bindings
- `g` - Commit all changes and push to remote
- `p` - Pull changes from remote

## Configuration File

Your Git settings are stored in `~/.config/rnotes/config.json`:
```json
{
  "root_directory": "/home/user/rnotes",
  "editor": "vim",
  "git_enabled": true,
  "git_repository": "https://github.com/yourusername/your-notes-repo.git",
  "git_username": "yourusername",
  "git_email": "your-email@example.com"
}
```

## Security Note

RNotes uses GitHub CLI for authentication, which is the recommended secure method for accessing GitHub repositories. Make sure you have `gh` installed and properly authenticated with `gh auth login`. Never commit sensitive information to your notes repository.
