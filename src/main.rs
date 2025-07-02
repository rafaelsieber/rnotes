use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    fs,
    io,
    path::PathBuf,
    process::Command,
};

mod config;
mod file_tree;
mod git;
mod markdown;

use config::Config;
use file_tree::FileTree;
use git::GitManager;
use markdown::MarkdownRenderer;

#[derive(Debug, Clone, PartialEq)]
enum AppMode {
    Normal,
    Config,
    Rename,
    DeleteConfirm,
    LineNavigation,
}

pub struct App {
    config: Config,
    file_tree: FileTree,
    current_content: String,
    current_file: Option<PathBuf>,
    mode: AppMode,
    config_input: String,
    config_field: usize, // 0 = root_dir, 1 = editor, 2 = git_enabled, 3 = git_repo, 4 = git_username, 5 = git_email
    rename_input: String,
    delete_target: Option<PathBuf>,
    // Line navigation fields
    content_lines: Vec<String>,
    rendered_lines: Vec<ratatui::text::Line<'static>>, // For formatted line navigation
    line_selection: usize,
    should_quit: bool,
    git_manager: GitManager,
    markdown_renderer: MarkdownRenderer,
}

impl App {
    pub fn new() -> Result<App> {
        let config = Config::load_or_create()?;
        let file_tree = FileTree::new(&config.root_directory)?;
        let git_manager = GitManager::new(config.clone());
        
        // Initialize Git repository if enabled
        if config.git_enabled {
            if let Err(e) = git_manager.init_repository() {
                eprintln!("Warning: Failed to initialize Git repository: {}", e);
            } else {
                // Perform initial git pull to sync with remote (quiet mode)
                if let Err(e) = git_manager.pull_changes_with_feedback(false) {
                    eprintln!("Warning: Failed to pull initial changes: {}", e);
                }
            }
        }
        
        // Create welcome file if it doesn't exist
        let welcome_path = config.root_directory.join("welcome.md");
        if !welcome_path.exists() {
            fs::write(
                &welcome_path,
                "# Welcome to RNotes!\n\nThis is your markdown notes manager.\n\n## Features:\n- Navigate through markdown files\n- Edit files with your preferred editor\n- VIM-like interface\n- Git integration for syncing notes\n\n## Usage:\n- Use arrow keys or j/k to navigate\n- Press Enter to edit a file\n- Press 'n' to create a new file\n- Press 'c' to open configuration\n- Press 'q' to quit\n- Press 'g' for Git operations\n\nHappy note-taking!",
            )?;
        }

        let mut app = App {
            config,
            file_tree,
            current_content: String::new(),
            current_file: None,
            mode: AppMode::Normal,
            config_input: String::new(),
            config_field: 0,
            rename_input: String::new(),
            delete_target: None,
            content_lines: Vec::new(),
            rendered_lines: Vec::new(),
            line_selection: 0,
            should_quit: false,
            git_manager,
            markdown_renderer: MarkdownRenderer::new(),
        };
        
        // Load the first file's content automatically
        app.load_current_file_content()?;
        
        Ok(app)
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Force a clear and redraw to handle any terminal corruption
            terminal.clear()?;
            terminal.draw(|f| self.ui(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match self.mode {
                        AppMode::Normal => self.handle_normal_input(key.code)?,
                        AppMode::Config => self.handle_config_input(key.code)?,
                        AppMode::Rename => self.handle_rename_input(key.code)?,
                        AppMode::DeleteConfirm => self.handle_delete_confirm_input(key.code)?,
                        AppMode::LineNavigation => self.handle_line_navigation_input(key.code)?,
                    }
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_normal_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                self.file_tree.next();
                self.load_current_file_content()?;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.file_tree.previous();
                self.load_current_file_content()?;
            }
            KeyCode::Char(' ') | KeyCode::Right => {
                // Smart right arrow: expand folder or enter line navigation
                if let Some(selected_path) = self.file_tree.get_selected_path() {
                    if selected_path.is_dir() {
                        // Toggle folder expansion/collapse
                        self.file_tree.toggle_selected()?;
                    } else {
                        // Enter line navigation mode if a file is selected
                        self.enter_line_navigation_mode()?;
                    }
                } else {
                    // If nothing selected, try to toggle
                    self.file_tree.toggle_selected()?;
                }
            }
            KeyCode::Char('i') => self.edit_current_file()?,
            KeyCode::Char('n') => self.create_new_file()?,
            KeyCode::Char('r') => self.start_rename()?,
            KeyCode::Char('x') => self.start_delete()?,
            KeyCode::Char('d') => self.create_new_folder()?,
            KeyCode::Char('c') => {
                self.mode = AppMode::Config;
                self.config_input = self.config.root_directory.to_string_lossy().to_string();
                self.config_field = 0;
            }
            KeyCode::Char('g') => {
                // Git push (commit and push changes)
                self.perform_git_push()?;
            }
            KeyCode::Char('p') => {
                // Git pull changes
                self.perform_git_pull()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_config_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.config_input.clear();
            }
            KeyCode::Tab => {
                self.save_current_config_field();
                self.config_field = (self.config_field + 1) % 6; // Now 6 fields total
                self.load_current_config_field();
            }
            KeyCode::Enter => {
                // Save current field and exit config mode
                self.save_current_config_field();
                
                self.config.save()?;
                
                // Update git manager with new config
                self.git_manager = GitManager::new(self.config.clone());
                
                // Initialize Git repository if enabled
                if self.config.git_enabled {
                    if let Err(e) = self.git_manager.init_repository() {
                        eprintln!("Warning: Failed to initialize Git repository: {}", e);
                    }
                }
                
                self.file_tree = FileTree::new(&self.config.root_directory)?;
                self.mode = AppMode::Normal;
                self.config_input.clear();
            }
            KeyCode::Char(c) => {
                if self.config_field == 2 { // Git enabled field
                    // For boolean field, toggle on any character input
                    self.config.git_enabled = !self.config.git_enabled;
                    self.config_input = self.config.git_enabled.to_string();
                } else {
                    self.config_input.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.config_field != 2 { // Don't allow backspace on boolean field
                    self.config_input.pop();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_rename_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.rename_input.clear();
            }
            KeyCode::Enter => {
                self.perform_rename()?;
                self.mode = AppMode::Normal;
                self.rename_input.clear();
            }
            KeyCode::Char(c) => {
                self.rename_input.push(c);
            }
            KeyCode::Backspace => {
                self.rename_input.pop();
            }
            _ => {}
        }
        Ok(())
    }

    fn start_rename(&mut self) -> Result<()> {
        if let Some(path) = self.file_tree.get_selected_path() {
            self.mode = AppMode::Rename;
            if path.is_dir() {
                // For directories, use the full name
                self.rename_input = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
            } else {
                // For files, use the stem (without extension)
                self.rename_input = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
            }
        }
        Ok(())
    }

    fn perform_rename(&mut self) -> Result<()> {
        if let Some(current_path) = self.file_tree.get_selected_path() {
            let current_path = current_path.clone(); // Clone to avoid borrow issues
            if !self.rename_input.is_empty() {
                // Save current tree state
                let expanded_dirs = self.file_tree.get_expansion_state();
                
                let parent = current_path.parent().unwrap_or(&self.config.root_directory);
                
                let new_filename = if current_path.is_dir() {
                    // For directories, use the name as-is
                    self.rename_input.clone()
                } else {
                    // For files, preserve the extension
                    let extension = current_path.extension().unwrap_or_default();
                    if extension.is_empty() {
                        self.rename_input.clone()
                    } else {
                        format!("{}.{}", self.rename_input, extension.to_string_lossy())
                    }
                };
                
                let new_path = parent.join(&new_filename);
                
                if !new_path.exists() {
                    fs::rename(&current_path, &new_path)?;
                    
                    // Update current_file if it was the renamed item
                    if Some(&current_path) == self.current_file.as_ref() {
                        if new_path.is_file() {
                            self.current_file = Some(new_path.clone());
                            self.load_current_file_content()?;
                        } else {
                            self.current_file = None;
                            self.current_content.clear();
                        }
                    }
                    
                    // Refresh file tree while preserving state and selecting the renamed item
                    self.file_tree.refresh_with_state(expanded_dirs, Some(new_path))?;
                }
            }
        }
        Ok(())
    }

    fn load_current_file_content(&mut self) -> Result<()> {
        if let Some(file_path) = self.file_tree.get_selected_file() {
            self.current_file = Some(file_path.clone());
            if file_path.extension().and_then(|s| s.to_str()) == Some("md") {
                match fs::read_to_string(&file_path) {
                    Ok(content) => {
                        self.current_content = content.clone();
                        self.content_lines = content.lines().map(|s| s.to_string()).collect();
                        
                        // Generate formatted lines for line navigation
                        match self.markdown_renderer.parse_markdown(&content) {
                            Ok(elements) => {
                                let rendered_text = self.markdown_renderer.render_to_text(&elements);
                                self.rendered_lines = rendered_text.lines.into_iter().collect();
                            }
                            Err(_) => {
                                // Fallback to plain text lines
                                self.rendered_lines = self.content_lines.iter()
                                    .map(|line| Line::from(line.clone()))
                                    .collect();
                            }
                        }
                        
                        self.line_selection = 0;
                    },
                    Err(_) => {
                        self.current_content = "Error reading file".to_string();
                        self.content_lines = vec!["Error reading file".to_string()];
                        self.rendered_lines = vec![Line::from("Error reading file".to_string())];
                        self.line_selection = 0;
                    }
                }
            } else {
                self.current_content = "Not a markdown file".to_string();
                self.content_lines = vec!["Not a markdown file".to_string()];
                self.rendered_lines = vec![Line::from("Not a markdown file".to_string())];
                self.line_selection = 0;
            }
        } else {
            self.current_content.clear();
            self.content_lines.clear();
            self.rendered_lines.clear();
            self.current_file = None;
            self.line_selection = 0;
        }
        Ok(())
    }

    fn edit_current_file(&mut self) -> Result<()> {
        if let Some(file_path) = &self.current_file {
            // Temporarily disable raw mode for the editor
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;

            let status = Command::new(&self.config.editor)
                .arg(file_path)
                .status()?;

            // Re-enable raw mode and properly restore terminal
            enable_raw_mode()?;
            execute!(io::stdout(), EnterAlternateScreen)?;
            
            // Clear the screen to avoid corruption
            execute!(io::stdout(), Clear(ClearType::All))?;

            if status.success() {
                // Reload the file content after editing
                self.load_current_file_content()?;
            } else {
                eprintln!("Editor exited with error");
            }
        }
        Ok(())
    }

    fn create_new_file(&mut self) -> Result<()> {
        // Save current tree state before creating the file
        let expanded_dirs = self.file_tree.get_expansion_state();
        
        // Determine the target directory
        let target_dir = if let Some(selected_path) = self.file_tree.get_selected_path() {
            if selected_path.is_dir() {
                // If a directory is selected, create the file inside it
                // Make sure this directory is expanded after refresh
                selected_path.clone()
            } else {
                // If a file is selected, create the file in its parent directory
                selected_path.parent().unwrap_or(&self.config.root_directory).to_path_buf()
            }
        } else {
            // If nothing is selected, use the root directory
            self.config.root_directory.clone()
        };
        
        // Simple implementation - create a file with timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let filename = format!("note_{}.md", timestamp);
        let file_path = target_dir.join(&filename);
        
        fs::write(&file_path, "# New Note\n\nWrite your notes here...\n")?;
        
        // If we created a file in a directory, make sure that directory stays expanded
        let mut final_expanded_dirs = expanded_dirs;
        if target_dir != self.config.root_directory && !final_expanded_dirs.contains(&target_dir) {
            final_expanded_dirs.push(target_dir.clone());
        }
        
        // Refresh file tree while preserving state, and try to select the new file
        self.file_tree.refresh_with_state(final_expanded_dirs, Some(file_path.clone()))?;
        
        // Update current file to the newly created one
        self.current_file = Some(file_path);
        self.load_current_file_content()?;
        
        Ok(())
    }

    fn create_new_folder(&mut self) -> Result<()> {
        // Save current tree state before creating the folder
        let expanded_dirs = self.file_tree.get_expansion_state();
        
        // Determine the target directory
        let target_dir = if let Some(selected_path) = self.file_tree.get_selected_path() {
            if selected_path.is_dir() {
                // If a directory is selected, create the folder inside it
                selected_path.clone()
            } else {
                // If a file is selected, create the folder in its parent directory
                selected_path.parent().unwrap_or(&self.config.root_directory).to_path_buf()
            }
        } else {
            // If nothing is selected, use the root directory
            self.config.root_directory.clone()
        };
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let folder_name = format!("folder_{}", timestamp);
        let folder_path = target_dir.join(&folder_name);
        
        fs::create_dir(&folder_path)?;
        
        // If we created a folder in a directory, make sure that directory stays expanded
        let mut final_expanded_dirs = expanded_dirs;
        if target_dir != self.config.root_directory && !final_expanded_dirs.contains(&target_dir) {
            final_expanded_dirs.push(target_dir.clone());
        }
        
        // Refresh file tree while preserving state, and try to select the new folder
        self.file_tree.refresh_with_state(final_expanded_dirs, Some(folder_path))?;
        
        Ok(())
    }

    fn handle_delete_confirm_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.perform_delete()?;
                self.mode = AppMode::Normal;
                self.delete_target = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.delete_target = None;
            }
            _ => {}
        }
        Ok(())
    }

    fn start_delete(&mut self) -> Result<()> {
        if let Some(path) = self.file_tree.get_selected_path() {
            self.delete_target = Some(path.clone());
            self.mode = AppMode::DeleteConfirm;
        }
        Ok(())
    }

    fn perform_delete(&mut self) -> Result<()> {
        if let Some(target_path) = &self.delete_target {
            let target_path = target_path.clone(); // Clone to avoid borrow issues
            // Save current tree state
            let expanded_dirs = self.file_tree.get_expansion_state();
            let parent_dir = target_path.parent();
            
            if target_path.is_dir() {
                // For directories, remove recursively
                std::fs::remove_dir_all(&target_path)?;
            } else {
                // For files, remove the file
                std::fs::remove_file(&target_path)?;
            }
            
            // If we deleted the currently viewed file, clear the content
            if Some(&target_path) == self.current_file.as_ref() {
                self.current_file = None;
                self.current_content.clear();
            }
            
            // Try to select the parent directory after deletion
            let selection_target = parent_dir.map(|p| p.to_path_buf());
            
            // Refresh the file tree while preserving expansion state
            self.file_tree.refresh_with_state(expanded_dirs, selection_target)?;
            
            // Try to load content for the new selection if any
            self.load_current_file_content()?;
        }
        Ok(())
    }

    fn handle_line_navigation_input(&mut self, key_code: KeyCode) -> Result<()> {
        match key_code {
            KeyCode::Esc | KeyCode::Left => {
                // Exit line navigation mode
                self.mode = AppMode::Normal;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.line_selection < self.rendered_lines.len().saturating_sub(1) {
                    self.line_selection += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.line_selection > 0 {
                    self.line_selection -= 1;
                }
            }
            KeyCode::Char('y') => {
                self.copy_current_line()?;
            }
            KeyCode::Char('i') => {
                // Edit file from line navigation mode
                self.mode = AppMode::Normal;
                self.edit_current_file()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn enter_line_navigation_mode(&mut self) -> Result<()> {
        if self.current_file.is_some() && !self.current_content.is_empty() {
            // Use the pre-processed lines (content_lines for plain text copy, rendered_lines for display)
            self.line_selection = 0;
            self.mode = AppMode::LineNavigation;
        }
        Ok(())
    }

    fn copy_current_line(&mut self) -> Result<()> {
        if let Some(line) = self.content_lines.get(self.line_selection) {
            match arboard::Clipboard::new() {
                Ok(mut clipboard) => {
                    match clipboard.set_text(line.clone()) {
                        Ok(_) => {
                            // Successfully copied to clipboard
                            // We could add a status message here in the future
                        }
                        Err(e) => {
                            // Failed to copy to clipboard
                            eprintln!("Failed to copy to clipboard: {}", e);
                        }
                    }
                }
                Err(e) => {
                    // Failed to create clipboard
                    eprintln!("Failed to create clipboard: {}", e);
                }
            }
        }
        Ok(())
    }

    fn save_current_config_field(&mut self) {
        match self.config_field {
            0 => {
                if let Ok(path) = PathBuf::from(&self.config_input).canonicalize() {
                    self.config.root_directory = path;
                }
            }
            1 => {
                self.config.editor = self.config_input.clone();
            }
            2 => {
                // Git enabled is handled in the input handler
            }
            3 => {
                if self.config_input.trim().is_empty() {
                    self.config.git_repository = None;
                } else {
                    self.config.git_repository = Some(self.config_input.clone());
                }
            }
            4 => {
                if self.config_input.trim().is_empty() {
                    self.config.git_username = None;
                } else {
                    self.config.git_username = Some(self.config_input.clone());
                }
            }
            5 => {
                if self.config_input.trim().is_empty() {
                    self.config.git_email = None;
                } else {
                    self.config.git_email = Some(self.config_input.clone());
                }
            }
            _ => {}
        }
    }

    fn load_current_config_field(&mut self) {
        self.config_input = match self.config_field {
            0 => self.config.root_directory.to_string_lossy().to_string(),
            1 => self.config.editor.clone(),
            2 => self.config.git_enabled.to_string(),
            3 => self.config.git_repository.clone().unwrap_or_default(),
            4 => self.config.git_username.clone().unwrap_or_default(),
            5 => self.config.git_email.clone().unwrap_or_default(),
            _ => String::new(),
        };
    }

    fn perform_git_push(&mut self) -> Result<()> {
        if !self.config.git_enabled {
            return Ok(());
        }

        // Commit current changes and push
        if let Err(e) = self.git_manager.commit_and_push() {
            eprintln!("Git push failed: {}", e);
        }

        Ok(())
    }

    fn perform_git_pull(&mut self) -> Result<()> {
        if !self.config.git_enabled {
            return Ok(());
        }

        // Pull changes from remote
        if let Err(e) = self.git_manager.pull_changes() {
            eprintln!("Git pull failed: {}", e);
        } else {
            // Refresh the file tree after pulling changes
            let expanded_dirs = self.file_tree.get_expansion_state();
            let selected_path = self.file_tree.get_selected_path().map(|p| p.clone());
            self.file_tree.refresh_with_state(expanded_dirs, selected_path)?;
            self.load_current_file_content()?;
        }

        Ok(())
    }

    fn ui(&mut self, f: &mut Frame) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Top bar
                Constraint::Min(3),    // Main content
                Constraint::Length(1), // Footer
            ])
            .split(f.size());

        // Render top bar
        self.render_top_bar(f, main_chunks[0]);

        // Main content area
        if self.mode == AppMode::Config {
            self.render_config_screen(f, main_chunks[1]);
        } else if self.mode == AppMode::Rename {
            self.render_rename_screen(f, main_chunks[1]);
        } else if self.mode == AppMode::DeleteConfirm {
            self.render_delete_confirm_screen(f, main_chunks[1]);
        } else if self.mode == AppMode::LineNavigation {
            self.render_line_navigation_screen(f, main_chunks[1]);
        } else {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(main_chunks[1]);

            // Create the items vector first
            let file_items = self.file_tree.get_items();
            let items: Vec<ListItem> = file_items
                .iter()
                .map(|item| {
                    let style = if item.contains("▶") || item.contains("▼") {
                        // Directory
                        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                    } else if item.ends_with(".md") {
                        // Markdown file
                        Style::default().fg(Color::Green)
                    } else {
                        // Other files
                        Style::default().fg(Color::Gray)
                    };
                    ListItem::new(item.as_str()).style(style)
                })
                .collect();

            let list = List::new(items)
                .block(Block::default().title("Files").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[0], self.file_tree.get_state_mut());
            
            // Render content
            let title = if let Some(file_path) = &self.current_file {
                format!("Content - {}", file_path.file_name().unwrap().to_string_lossy())
            } else {
                "Content".to_string()
            };

            // Use markdown rendering for .md files, plain text for others
            if let Some(file_path) = &self.current_file {
                if file_path.extension().and_then(|s| s.to_str()) == Some("md") && !self.current_content.is_empty() {
                    // Parse and render markdown
                    match self.markdown_renderer.parse_markdown(&self.current_content) {
                        Ok(elements) => {
                            let rendered_text = self.markdown_renderer.render_to_text(&elements);
                            let paragraph = Paragraph::new(rendered_text)
                                .block(Block::default().title(title.as_str()).borders(Borders::ALL))
                                .wrap(Wrap { trim: true })
                                .scroll((0, 0));
                            f.render_widget(paragraph, chunks[1]);
                        }
                        Err(_) => {
                            // Fallback to plain text if markdown parsing fails
                            let paragraph = Paragraph::new(self.current_content.as_str())
                                .block(Block::default().title(title.as_str()).borders(Borders::ALL))
                                .wrap(Wrap { trim: true });
                            f.render_widget(paragraph, chunks[1]);
                        }
                    }
                } else {
                    // Plain text rendering for non-markdown files
                    let paragraph = Paragraph::new(self.current_content.as_str())
                        .block(Block::default().title(title.as_str()).borders(Borders::ALL))
                        .wrap(Wrap { trim: true });
                    f.render_widget(paragraph, chunks[1]);
                }
            } else {
                // No file selected
                let paragraph = Paragraph::new("No file selected")
                    .block(Block::default().title("Content").borders(Borders::ALL))
                    .style(Style::default().fg(Color::Gray));
                f.render_widget(paragraph, chunks[1]);
            }
        }

        // Render footer
        self.render_footer(f, main_chunks[2]);
    }



    fn render_config_screen(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Root directory
                Constraint::Length(3), // Editor
                Constraint::Length(3), // Git enabled
                Constraint::Length(3), // Git repository
                Constraint::Length(3), // Git username
                Constraint::Length(3), // Git email
                Constraint::Min(1),    // Help
            ])
            .split(area);

        // Title
        let title = Paragraph::new("Configuration")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(title, chunks[0]);

        // Root directory field
        let root_dir_style = if self.config_field == 0 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let root_dir_content = if self.config_field == 0 {
            self.config_input.as_str()
        } else {
            &self.config.root_directory.to_string_lossy()
        };
        let root_dir = Paragraph::new(root_dir_content)
            .block(Block::default().title("Root Directory").borders(Borders::ALL))
            .style(root_dir_style);
        f.render_widget(root_dir, chunks[1]);

        // Editor field
        let editor_style = if self.config_field == 1 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let editor_content = if self.config_field == 1 {
            self.config_input.as_str()
        } else {
            self.config.editor.as_str()
        };
        let editor = Paragraph::new(editor_content)
            .block(Block::default().title("Editor").borders(Borders::ALL))
            .style(editor_style);
        f.render_widget(editor, chunks[2]);

        // Git enabled field
        let git_enabled_style = if self.config_field == 2 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let git_enabled_content = if self.config_field == 2 {
            self.config_input.as_str()
        } else {
            if self.config.git_enabled { "true" } else { "false" }
        };
        let git_enabled = Paragraph::new(git_enabled_content)
            .block(Block::default().title("Git Enabled (any key to toggle)").borders(Borders::ALL))
            .style(git_enabled_style);
        f.render_widget(git_enabled, chunks[3]);

        // Git repository field
        let git_repo_style = if self.config_field == 3 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let git_repo_content = if self.config_field == 3 {
            self.config_input.as_str()
        } else {
            self.config.git_repository.as_deref().unwrap_or("")
        };
        let git_repo = Paragraph::new(git_repo_content)
            .block(Block::default().title("Git Repository URL").borders(Borders::ALL))
            .style(git_repo_style);
        f.render_widget(git_repo, chunks[4]);

        // Git username field
        let git_username_style = if self.config_field == 4 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let git_username_content = if self.config_field == 4 {
            self.config_input.as_str()
        } else {
            self.config.git_username.as_deref().unwrap_or("")
        };
        let git_username = Paragraph::new(git_username_content)
            .block(Block::default().title("Git Username").borders(Borders::ALL))
            .style(git_username_style);
        f.render_widget(git_username, chunks[5]);

        // Git email field
        let git_email_style = if self.config_field == 5 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let git_email_content = if self.config_field == 5 {
            self.config_input.as_str()
        } else {
            self.config.git_email.as_deref().unwrap_or("")
        };
        let git_email = Paragraph::new(git_email_content)
            .block(Block::default().title("Git Email").borders(Borders::ALL))
            .style(git_email_style);
        f.render_widget(git_email, chunks[6]);

        // Help text
        let help = Paragraph::new("Tab: Next field | Enter: Save & Exit | Esc: Cancel")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(help, chunks[7]);
    }

    fn render_top_bar(&self, f: &mut Frame, area: Rect) {
        let current_file_name = if let Some(file_path) = &self.current_file {
            file_path.file_name().unwrap().to_string_lossy().to_string()
        } else {
            "No file selected".to_string()
        };
        
        // Show current context for file creation
        let current_context = if let Some(selected_path) = self.file_tree.get_selected_path() {
            if selected_path.is_dir() {
                format!("📁 {}", selected_path.file_name().unwrap().to_string_lossy())
            } else {
                let parent = selected_path.parent().unwrap_or(&self.config.root_directory);
                if parent == &self.config.root_directory {
                    "📁 root".to_string()
                } else {
                    format!("📁 {}", parent.file_name().unwrap().to_string_lossy())
                }
            }
        } else {
            "📁 root".to_string()
        };
        
        let root_dir = self.config.root_directory.to_string_lossy();
        
        // Add Git status if enabled
        let git_status = if self.config.git_enabled {
            match self.git_manager.get_status() {
                Ok(status) => {
                    if status.has_changes() {
                        format!(" | Git: {} changes", status.modified + status.untracked)
                    } else {
                        " | Git: ✓".to_string()
                    }
                }
                Err(_) => " | Git: ⚠".to_string(),
            }
        } else {
            String::new()
        };
        
        let status_line = format!(" RNotes - {} | Current: {} | Root: {}{} ", 
                                current_file_name, current_context, root_dir, git_status);
        
        let paragraph = Paragraph::new(status_line.as_str())
            .style(Style::default().bg(Color::Blue).fg(Color::White));
        
        f.render_widget(paragraph, area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let footer_text = match self.mode {
            AppMode::Normal => {
                if self.config.git_enabled {
                    " j/k:Navigate | Space/→:Expand/Lines | i:Edit | n:New | r:Rename | x:Delete | d:Folder | c:Config | g:Push | p:Pull | q:Quit "
                } else {
                    " j/k:Navigate | Space/→:Expand/Lines | i:Edit | n:New | r:Rename | x:Delete | d:Folder | c:Config | q:Quit "
                }
            }
            AppMode::Config => " Tab:Next field | Enter:Save | Esc:Cancel ",
            AppMode::Rename => " Type new name | Enter:Confirm | Esc:Cancel ",
            AppMode::DeleteConfirm => " y:Yes, delete | n:No, cancel | Esc:Cancel ",
            AppMode::LineNavigation => " j/k:Navigate lines | y:Copy line | i:Edit | ←/Esc:Back ",
        };
        
        let paragraph = Paragraph::new(footer_text)
            .style(Style::default().bg(Color::Gray).fg(Color::Black));
        
        f.render_widget(paragraph, area);
    }

    fn render_rename_screen(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(1),
            ])
            .split(area);

        // Title
        let (current_name, item_type) = if let Some(path) = self.file_tree.get_selected_path() {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let type_str = if path.is_dir() { "Folder" } else { "File" };
            (name, type_str)
        } else {
            ("No item selected".to_string(), "Item")
        };
        
        let title = Paragraph::new(format!("Rename {}: {}", item_type, current_name))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(title, chunks[0]);

        // Input field
        let input = Paragraph::new(self.rename_input.as_str())
            .block(Block::default().title("New Name").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));
        f.render_widget(input, chunks[1]);
    }

    fn render_delete_confirm_screen(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Min(1),
            ])
            .split(area);

        // Confirmation message
        let (target_name, item_type) = if let Some(path) = &self.delete_target {
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let type_str = if path.is_dir() { "folder" } else { "file" };
            (name, type_str)
        } else {
            ("Unknown".to_string(), "item")
        };
        
        let warning_text = format!(
            "⚠️  DELETE CONFIRMATION  ⚠️\n\nAre you sure you want to delete this {}?\n\n📁 {}\n\nThis action cannot be undone!",
            item_type, target_name
        );
        
        let warning = Paragraph::new(warning_text.as_str())
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        f.render_widget(warning, chunks[0]);

        // Instructions
        let instructions = Paragraph::new("Press 'y' to DELETE or 'n' to CANCEL")
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(instructions, chunks[1]);
    }

    fn render_line_navigation_screen(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(area);

        // Create the items vector for file tree
        let file_items = self.file_tree.get_items();
        let items: Vec<ListItem> = file_items
            .iter()
            .map(|item| {
                let style = if item.contains("▶") || item.contains("▼") {
                    // Directory
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else if item.ends_with(".md") {
                    // Markdown file
                    Style::default().fg(Color::Green)
                } else {
                    // Other files
                    Style::default().fg(Color::Gray)
                };
                ListItem::new(item.as_str()).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Files").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        f.render_stateful_widget(list, chunks[0], self.file_tree.get_state_mut());
        
        // Render content with line navigation using formatted lines
        let title = if let Some(file_path) = &self.current_file {
            format!("Line Navigation - {}", file_path.file_name().unwrap().to_string_lossy())
        } else {
            "Line Navigation".to_string()
        };

        // Create line items with highlighting using rendered/formatted lines
        let line_items: Vec<ListItem> = self.rendered_lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let base_style = if i == self.line_selection {
                    Style::default().bg(Color::Blue)
                } else {
                    Style::default()
                };
                
                // Create a line with line number and preserve the formatting
                let line_number = format!("{:3}: ", i + 1);
                let mut spans = vec![Span::styled(
                    line_number,
                    Style::default().fg(Color::DarkGray),
                )];
                
                // Add the formatted line spans
                spans.extend(line.spans.iter().cloned());
                
                // Apply selection highlighting if needed
                if i == self.line_selection {
                    // Apply background color to all spans
                    for span in &mut spans {
                        span.style = span.style.bg(Color::Blue);
                    }
                }
                
                ListItem::new(Line::from(spans)).style(base_style)
            })
            .collect();

        let line_list = List::new(line_items)
            .block(Block::default().title(title.as_str()).borders(Borders::ALL))
            .highlight_style(Style::default()) // Don't override our custom highlighting
            .highlight_symbol(""); // Remove default highlight symbol since we're doing custom highlighting

        // Create a list state for line navigation
        let mut line_state = ratatui::widgets::ListState::default();
        line_state.select(Some(self.line_selection));

        f.render_stateful_widget(line_list, chunks[1], &mut line_state);
    }
}

fn main() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new()?;
    let res = app.run(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
