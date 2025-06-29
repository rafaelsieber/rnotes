use anyhow::Result;
use ratatui::widgets::ListState;
use std::{fs, path::PathBuf};

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub path: PathBuf,
    pub display_name: String,
    pub is_expanded: bool,
    pub is_dir: bool,
}

pub struct FileTree {
    items: Vec<TreeItem>,
    state: ListState,
    root_dir: PathBuf,
}

impl FileTree {
    pub fn new(root_dir: &PathBuf) -> Result<Self> {
        let mut tree = FileTree {
            items: Vec::new(),
            state: ListState::default(),
            root_dir: root_dir.clone(),
        };
        
        tree.build_tree()?;
        
        if !tree.items.is_empty() {
            tree.state.select(Some(0));
        }
        
        Ok(tree)
    }
    
    fn build_tree(&mut self) -> Result<()> {
        self.items.clear();
        let root_dir = self.root_dir.clone();
        if root_dir.exists() && root_dir.is_dir() {
            self.add_directory_contents(&root_dir, 0, &mut Vec::new())?;
        }
        Ok(())
    }
    
    fn add_directory_contents(&mut self, dir: &PathBuf, depth: usize, expanded_dirs: &mut Vec<PathBuf>) -> Result<()> {
        let mut entries: Vec<_> = fs::read_dir(dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                let path = entry.path();
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                
                // Filter out .git directory and other hidden directories/files starting with .
                if file_name.starts_with('.') {
                    return false;
                }
                
                // Only show directories or markdown files
                path.is_dir() || path.extension().and_then(|s| s.to_str()) == Some("md")
            })
            .collect();

        // Sort entries: directories first, then files, both alphabetically
        entries.sort_by(|a, b| {
            let a_path = a.path();
            let b_path = b.path();
            
            match (a_path.is_dir(), b_path.is_dir()) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a_path.file_name().cmp(&b_path.file_name()),
            }
        });

        for entry in entries {
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();

            let is_dir = path.is_dir();
            let is_expanded = expanded_dirs.contains(&path);
            
            // Create the display name with proper indentation
            let indent = "  ".repeat(depth);
            let prefix = if is_dir {
                if is_expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };
            
            let display_name = format!("{}{}{}", indent, prefix, name);

            self.items.push(TreeItem {
                path: path.clone(),
                display_name,
                is_expanded,
                is_dir,
            });

            // If it's a directory and expanded, recursively add its contents
            if is_dir && is_expanded {
                self.add_directory_contents(&path, depth + 1, expanded_dirs)?;
            }
        }
        
        Ok(())
    }
    
    pub fn get_items(&self) -> Vec<String> {
        self.items.iter().map(|item| item.display_name.clone()).collect()
    }
    
    pub fn get_state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }
    
    pub fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    
    pub fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
    
    pub fn get_selected_file(&self) -> Option<&PathBuf> {
        if let Some(i) = self.state.selected() {
            self.items.get(i).filter(|item| !item.is_dir).map(|item| &item.path)
        } else {
            None
        }
    }

    pub fn get_selected_path(&self) -> Option<&PathBuf> {
        if let Some(i) = self.state.selected() {
            self.items.get(i).map(|item| &item.path)
        } else {
            None
        }
    }
    
    pub fn toggle_selected(&mut self) -> Result<()> {
        if let Some(i) = self.state.selected() {
            if let Some(item) = self.items.get(i) {
                if item.is_dir {
                    let path = item.path.clone();
                    let was_expanded = item.is_expanded;
                    
                    // Build list of currently expanded directories
                    let mut expanded_dirs: Vec<PathBuf> = self.items
                        .iter()
                        .filter(|item| item.is_dir && item.is_expanded)
                        .map(|item| item.path.clone())
                        .collect();
                    
                    // Toggle the selected directory
                    if was_expanded {
                        expanded_dirs.retain(|p| p != &path);
                    } else {
                        expanded_dirs.push(path);
                    }
                    
                    // Rebuild the tree with new expansion state
                    let selected_path = self.items[i].path.clone();
                    let root_dir = self.root_dir.clone();
                    self.items.clear();
                    self.add_directory_contents(&root_dir, 0, &mut expanded_dirs)?;
                    
                    // Try to maintain selection on the same item
                    if let Some(new_index) = self.items.iter().position(|item| item.path == selected_path) {
                        self.state.select(Some(new_index));
                    }
                }
            }
        }
        Ok(())
    }
    
    pub fn get_expansion_state(&self) -> Vec<PathBuf> {
        self.items
            .iter()
            .filter(|item| item.is_dir && item.is_expanded)
            .map(|item| item.path.clone())
            .collect()
    }
    
    pub fn refresh_with_state(&mut self, expanded_dirs: Vec<PathBuf>, selected_path: Option<PathBuf>) -> Result<()> {
        self.items.clear();
        let root_dir = self.root_dir.clone();
        let mut expanded_dirs = expanded_dirs;
        self.add_directory_contents(&root_dir, 0, &mut expanded_dirs)?;
        
        // Try to maintain selection
        if let Some(target_path) = selected_path {
            if let Some(new_index) = self.items.iter().position(|item| item.path == target_path) {
                self.state.select(Some(new_index));
            } else {
                // If the exact path is not found, try to select something nearby
                if !self.items.is_empty() {
                    self.state.select(Some(0));
                }
            }
        } else if !self.items.is_empty() {
            self.state.select(Some(0));
        }
        
        Ok(())
    }
}
