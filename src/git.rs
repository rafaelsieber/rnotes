use anyhow::{Result, Context};
use git2::{Repository, Signature};
use std::path::PathBuf;
use crate::config::Config;

pub struct GitManager {
    repo_path: PathBuf,
    config: Config,
}

impl GitManager {
    pub fn new(config: Config) -> Self {
        Self {
            repo_path: config.root_directory.clone(),
            config,
        }
    }

    /// Initialize a new Git repository in the notes directory
    pub fn init_repository(&self) -> Result<()> {
        if !self.config.git_enabled {
            return Ok(());
        }

        // Check if already a git repository
        if Repository::open(&self.repo_path).is_ok() {
            return Ok(());
        }

        // Initialize repository
        let repo = Repository::init(&self.repo_path)
            .context("Failed to initialize Git repository")?;

        // Create initial .gitignore if it doesn't exist
        let gitignore_path = self.repo_path.join(".gitignore");
        if !gitignore_path.exists() {
            let gitignore_content = "# RNotes Git ignore\n*.tmp\n*.bak\n*~\n.DS_Store\nThumbs.db\n";
            std::fs::write(&gitignore_path, gitignore_content)
                .context("Failed to create .gitignore")?;
        }

        // Set up remote if configured
        if let Some(remote_url) = &self.config.git_repository {
            repo.remote("origin", remote_url)
                .context("Failed to add remote origin")?;
        }

        Ok(())
    }

    /// Add all changes and commit with a generic message
    pub fn commit_and_push(&self) -> Result<()> {
        if !self.config.git_enabled {
            return Err(anyhow::anyhow!("Git integration is not enabled"));
        }

        let repo = Repository::open(&self.repo_path)
            .context("Failed to open Git repository")?;

        let mut index = repo.index()
            .context("Failed to get repository index")?;

        // Add all files
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .context("Failed to add files to index")?;

        index.write()
            .context("Failed to write index")?;

        // Check if there are any changes to commit
        let tree_id = index.write_tree()
            .context("Failed to write tree")?;
        let tree = repo.find_tree(tree_id)
            .context("Failed to find tree")?;

        // Get HEAD commit if it exists
        let parent_commit = match repo.head() {
            Ok(head) => {
                let oid = head.target().context("Failed to get HEAD target")?;
                Some(repo.find_commit(oid).context("Failed to find HEAD commit")?)
            }
            Err(_) => None, // First commit
        };

        // Check if there are actually changes to commit
        let has_changes = if let Some(parent) = &parent_commit {
            let parent_tree = parent.tree().context("Failed to get parent tree")?;
            let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)
                .context("Failed to create diff")?;
            diff.deltas().len() > 0
        } else {
            // First commit, check if there are any files
            tree.len() > 0
        };

        if has_changes {
            // Create signature
            let signature = self.create_signature()?;

            // Create commit message with timestamp
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
            let commit_message = format!("Manual commit from RNotes - {}", timestamp);

            // Create the commit
            let parents: Vec<&git2::Commit> = parent_commit.as_ref().map_or(vec![], |c| vec![c]);
            
            repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &commit_message,
                &tree,
                &parents,
            ).context("Failed to create commit")?;

            println!("✓ Changes committed successfully");

            // Push changes if remote is configured
            if self.config.git_repository.is_some() {
                println!("→ Pushing to remote repository...");
                self.push_changes()?;
                println!("✓ Successfully pushed to remote repository");
            } else {
                println!("⚠ No remote repository configured");
            }
        } else {
            println!("ℹ No changes to commit");
        }

        Ok(())
    }

    /// Push changes to remote repository
    pub fn push_changes(&self) -> Result<()> {
        if !self.config.git_enabled || self.config.git_repository.is_none() {
            return Err(anyhow::anyhow!("Git not enabled or no repository configured"));
        }

        let repo = Repository::open(&self.repo_path)
            .context("Failed to open Git repository")?;

        // Try to get the remote - first "origin", then "rnotes", then the first available remote
        let mut remote = repo.find_remote("origin")
            .or_else(|_| repo.find_remote("rnotes"))
            .or_else(|_| {
                // Get the first available remote
                let remotes = repo.remotes()?;
                if let Some(remote_name) = remotes.get(0) {
                    repo.find_remote(remote_name)
                } else {
                    Err(git2::Error::from_str("No remote repositories found"))
                }
            })
            .context("Failed to find any remote repository")?;

        // Set up callbacks for GitHub CLI authentication
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|url, username_from_url, allowed_types| {
            // Try different credential types in order of preference
            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                // Use credential helper (works with gh auth)
                if let Ok(config) = git2::Config::open_default() {
                    if let Ok(cred) = git2::Cred::credential_helper(&config, url, username_from_url) {
                        return Ok(cred);
                    }
                }
            }
            
            if allowed_types.contains(git2::CredentialType::DEFAULT) {
                if let Ok(cred) = git2::Cred::default() {
                    return Ok(cred);
                }
            }

            // Fallback to username
            git2::Cred::username(username_from_url.unwrap_or("git"))
        });

        // Add progress callback for feedback
        callbacks.push_update_reference(|refname, status| {
            match status {
                Some(msg) => println!("Push failed for {}: {}", refname, msg),
                None => println!("Successfully updated {}", refname),
            }
            Ok(())
        });

        // Push to remote
        let mut push_options = git2::PushOptions::new();
        push_options.remote_callbacks(callbacks);

        let result = remote.push(&["refs/heads/main:refs/heads/main"], Some(&mut push_options))
            .or_else(|_| {
                // Try master branch if main doesn't work
                remote.push(&["refs/heads/master:refs/heads/master"], Some(&mut push_options))
            });

        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                Err(anyhow::anyhow!(
                    "Failed to push to remote repository: {}. \n\
                     Make sure you have GitHub CLI installed and authenticated:\n\
                     - Install: sudo apt install gh\n\
                     - Login: gh auth login\n\
                     - Verify: gh auth status", 
                    e
                ))
            }
        }
    }

    /// Pull changes from remote repository
    pub fn pull_changes(&self) -> Result<()> {
        self.pull_changes_with_feedback(true)
    }

    /// Pull changes from remote repository with optional feedback
    pub fn pull_changes_with_feedback(&self, show_feedback: bool) -> Result<()> {
        if !self.config.git_enabled || self.config.git_repository.is_none() {
            return Ok(());
        }

        let repo = Repository::open(&self.repo_path)
            .context("Failed to open Git repository")?;

        // Fetch from remote - try "origin" first, then "rnotes", then first available
        let mut remote = repo.find_remote("origin")
            .or_else(|_| repo.find_remote("rnotes"))
            .or_else(|_| {
                // Get the first available remote
                let remotes = repo.remotes()?;
                if let Some(remote_name) = remotes.get(0) {
                    repo.find_remote(remote_name)
                } else {
                    Err(git2::Error::from_str("No remote repositories found"))
                }
            })
            .context("Failed to find any remote repository")?;

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, allowed_types| {
            if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT) {
                // Use git credential helper (works with gh auth)
                git2::Cred::credential_helper(&git2::Config::open_default().unwrap(), _url, username_from_url)
            } else if allowed_types.contains(git2::CredentialType::DEFAULT) {
                git2::Cred::default()
            } else {
                git2::Cred::username(username_from_url.unwrap_or("git"))
            }
        });

        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], 
                    Some(&mut fetch_options), 
                    None)
            .context("Failed to fetch from remote. Make sure 'gh auth login' is configured.")?;

        if show_feedback {
            println!("✓ Fetched latest changes from remote");
        }

        // Perform merge (simple fast-forward merge)
        let fetch_head = repo.find_reference("FETCH_HEAD")
            .context("Failed to find FETCH_HEAD")?;
        let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)
            .context("Failed to get fetch commit")?;

        // Analyze merge
        let analysis = repo.merge_analysis(&[&fetch_commit])
            .context("Failed to analyze merge")?;

        if analysis.0.is_fast_forward() {
            // Fast-forward merge
            let refname = format!("refs/heads/{}", 
                                repo.head()?.shorthand().unwrap_or("main"));
            let mut reference = repo.find_reference(&refname)
                .context("Failed to find branch reference")?;
            reference.set_target(fetch_commit.id(), "Fast-forward")
                .context("Failed to set target for fast-forward")?;
            repo.set_head(&refname)
                .context("Failed to set HEAD")?;
            repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                .context("Failed to checkout HEAD")?;
            
            if show_feedback {
                println!("✓ Fast-forward merge completed");
            }
        } else if analysis.0.is_normal() {
            // Normal merge would be more complex, for now just warn
            if show_feedback {
                eprintln!("Warning: Manual merge required. Please resolve conflicts manually.");
            }
        } else if analysis.0.is_up_to_date() {
            if show_feedback {
                println!("✓ Already up to date");
            }
        }

        Ok(())
    }

    /// Create a signature for commits
    fn create_signature(&self) -> Result<Signature> {
        let name = self.config.git_username.as_deref().unwrap_or("RNotes User");
        let email = self.config.git_email.as_deref().unwrap_or("rnotes@localhost");
        
        Signature::now(name, email)
            .context("Failed to create Git signature")
    }

    /// Check if the directory is a Git repository
    pub fn is_git_repository(&self) -> bool {
        Repository::open(&self.repo_path).is_ok()
    }

    /// Get the current Git status (number of changed files)
    pub fn get_status(&self) -> Result<GitStatus> {
        if !self.config.git_enabled {
            return Ok(GitStatus::default());
        }

        let repo = Repository::open(&self.repo_path)
            .context("Failed to open Git repository")?;

        let statuses = repo.statuses(None)
            .context("Failed to get repository status")?;

        let mut modified = 0;
        let mut untracked = 0;
        let mut staged = 0;

        for entry in statuses.iter() {
            let status = entry.status();
            if status.contains(git2::Status::WT_MODIFIED) || 
               status.contains(git2::Status::WT_DELETED) {
                modified += 1;
            }
            if status.contains(git2::Status::WT_NEW) {
                untracked += 1;
            }
            if status.contains(git2::Status::INDEX_MODIFIED) || 
               status.contains(git2::Status::INDEX_NEW) || 
               status.contains(git2::Status::INDEX_DELETED) {
                staged += 1;
            }
        }

        Ok(GitStatus {
            modified,
            untracked,
            staged,
            has_remote: self.config.git_repository.is_some(),
        })
    }
}

#[derive(Debug, Default)]
pub struct GitStatus {
    pub modified: usize,
    pub untracked: usize,
    pub staged: usize,
    pub has_remote: bool,
}

impl GitStatus {
    pub fn has_changes(&self) -> bool {
        self.modified > 0 || self.untracked > 0 || self.staged > 0
    }
}
