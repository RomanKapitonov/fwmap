use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use cargo_metadata::MetadataCommand;

#[derive(Debug, Clone)]
pub struct WorkspaceLayout {
    pub root: Utf8PathBuf,
    pub target_dir: Utf8PathBuf,
}

impl WorkspaceLayout {
    pub fn discover() -> Result<Self> {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .context("cargo metadata failed; run fwmap from within a cargo workspace")?;
        Ok(Self {
            root: metadata.workspace_root,
            target_dir: metadata.target_directory,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_returns_fwmap_as_workspace_root_when_run_from_fwmap() {
        let layout = WorkspaceLayout::discover().expect("cargo_metadata should succeed");
        assert!(
            layout.root.ends_with("fwmap"),
            "expected workspace root to end with 'fwmap', got {}",
            layout.root
        );
        assert!(
            layout.target_dir.as_str().contains("target"),
            "expected target_dir to contain 'target', got {}",
            layout.target_dir
        );
    }
}
