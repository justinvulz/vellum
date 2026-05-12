use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

pub struct GitSync {
    pub enabled: bool,
    pub status: String,
}

impl Default for GitSync {
    fn default() -> Self {
        Self {
            enabled: false,
            status: "disabled".into(),
        }
    }
}

impl GitSync {
    pub fn init_repo(&mut self, root: &Path) -> Result<()> {
        let output = Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()
            .context("running git init")?;
        self.status = if output.status.success() {
            "initialized".into()
        } else {
            String::from_utf8_lossy(&output.stderr).into_owned()
        };
        Ok(())
    }

    pub fn commit_all(&mut self, root: &Path, message: &str) -> Result<()> {
        let add = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()
            .context("git add")?;
        if !add.status.success() {
            self.status = String::from_utf8_lossy(&add.stderr).into_owned();
            return Ok(());
        }
        let commit = Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(root)
            .output()
            .context("git commit")?;
        self.status = if commit.status.success() {
            format!("committed: {message}")
        } else {
            String::from_utf8_lossy(&commit.stderr).into_owned()
        };
        Ok(())
    }
}
