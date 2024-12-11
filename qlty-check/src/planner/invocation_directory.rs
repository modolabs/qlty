use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use qlty_analysis::{utils::fs::path_to_native_string, WorkspaceEntry};
use qlty_config::config::{DriverDef, InvocationDirectoryType, PluginDef};

use crate::Tool;

#[derive(Debug, Clone)]
pub struct InvocationDirectoryPlanner {
    pub driver: DriverDef,
    pub plugin: PluginDef,
    pub tool: Box<dyn Tool>,
    pub target_root: PathBuf,
}

impl InvocationDirectoryPlanner {
    pub fn compute(&self, target: &WorkspaceEntry) -> Result<PathBuf> {
        match self.driver.invocation_directory_def.kind {
            InvocationDirectoryType::Root => Ok(self.target_root.clone()),
            InvocationDirectoryType::TargetDirectory => self.target_directory(target),
            InvocationDirectoryType::RootOrParentWithAnyConfig => {
                self.root_or_parent_with_any_config(target)
            }
            InvocationDirectoryType::RootOrParentWith => {
                let target_directory = self.target_directory(target)?;
                let search_target = self.driver.invocation_directory_def.path.as_ref().unwrap();

                self.find_parent_folder_with(&target_directory, search_target)
            }
            InvocationDirectoryType::ToolDir => Ok(PathBuf::from(self.tool.directory())),
        }
    }

    fn find_parent_folder_with(
        &self,
        starting_directory: &Path,
        search_target: &String,
    ) -> Result<PathBuf> {
        let mut current_path = starting_directory.to_path_buf();

        loop {
            let potential_target = current_path.join(search_target);

            if std::fs::metadata(&potential_target).is_ok() {
                return Ok(current_path);
            }

            // loop until root is reached
            if current_path == self.target_root || !current_path.pop() {
                break;
            }
        }

        Ok(self.target_root.clone())
    }

    fn target_directory(&self, target: &WorkspaceEntry) -> Result<PathBuf> {
        let potential_dir = self.target_root.join(target.path.clone());

        if potential_dir.exists() {
            let metadata = std::fs::metadata(potential_dir.clone()).with_context(|| {
                format!(
                    "Failed to get metadata for potential target directory: {}",
                    path_to_native_string(&potential_dir)
                )
            })?;

            if metadata.is_dir() {
                Ok(potential_dir)
            } else {
                Ok(potential_dir.parent().unwrap().to_path_buf())
            }
        } else {
            Ok(self.target_root.clone())
        }
    }

    fn root_or_parent_with_any_config(&self, target: &WorkspaceEntry) -> Result<PathBuf> {
        let target_directory = self.target_directory(target)?;

        for plugin_config in &self.plugin.config_files {
            let search_target = plugin_config
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let path = self.find_parent_folder_with(&target_directory, &search_target)?;

            if path == self.target_root {
                continue;
            } else {
                return Ok(path);
            }
        }

        Ok(self.target_root.clone())
    }
}

#[cfg(test)]
mod test {
    use std::{fs::File, time::SystemTime};

    use crate::{
        planner::target::Target,
        tool::{
            command_builder::test::{reroute_tools_root, ENV_LOCK},
            null_tool::NullTool,
        },
    };

    use super::*;
    use qlty_analysis::WorkspaceEntryKind;
    use qlty_config::config::InvocationDirectoryDef;
    use qlty_test_utilities::git::sample_repo;
    use tempfile::TempDir;

    fn target_files(path: &str) -> Target {
        Target {
            path: PathBuf::from(path),
            kind: WorkspaceEntryKind::File,
            content_modified: SystemTime::now(),
            contents_size: 0,
            language_name: None,
        }
    }

    fn setup(
        invocation_directory_def: InvocationDirectoryDef,
    ) -> (TempDir, InvocationDirectoryPlanner) {
        // This is needed for tests using reroute_tools_root
        let _lock = ENV_LOCK.lock().unwrap_or_else(|err| {
            ENV_LOCK.clear_poison();
            err.into_inner()
        });

        let (temp_dir, _) = sample_repo();
        let temp_path = temp_dir.path().to_path_buf();

        let driver = DriverDef {
            invocation_directory_def,
            ..Default::default()
        };

        let invocation_directory_planner = InvocationDirectoryPlanner {
            driver: driver.clone(),
            plugin: PluginDef {
                config_files: vec!["config_file.json".into()],
                ..Default::default()
            },
            tool: Box::new(NullTool {
                plugin_name: "mock_plugin".to_string(),
                plugin: Default::default(),
            }),
            target_root: temp_path.clone(),
        };

        (temp_dir, invocation_directory_planner)
    }

    #[test]
    fn test_compute_root_invocation_directory() {
        let (temp_dir, invocation_directory_planner) = setup(InvocationDirectoryDef {
            kind: InvocationDirectoryType::Root,
            path: None,
        });

        let invocation_directory = invocation_directory_planner
            .compute(&target_files("lib/hello.rb"))
            .unwrap();

        assert_eq!(invocation_directory, temp_dir.path());
    }

    #[test]
    fn test_compute_root_or_parent_with_invocation_directory() {
        let (temp_dir, invocation_directory_planner) = setup(InvocationDirectoryDef {
            kind: InvocationDirectoryType::RootOrParentWith,
            path: Some("config_file.json".into()),
        });

        File::create(temp_dir.path().join("lib/config_file.json")).unwrap();

        let targets_results = vec![
            (target_files("lib/hello.rb"), temp_dir.path().join("lib")),
            (target_files("greetings.rb"), temp_dir.path().to_path_buf()),
            (
                target_files("lib/tasks/some.rb"),
                temp_dir.path().join("lib"),
            ),
        ];

        for (target, result) in targets_results {
            let invocation_directory = invocation_directory_planner.compute(&target).unwrap();
            assert_eq!(invocation_directory, result);
        }
    }

    #[test]
    fn test_compute_root_or_parent_with_any_config_invocation_directory() {
        let (temp_dir, invocation_directory_planner) = setup(InvocationDirectoryDef {
            kind: InvocationDirectoryType::RootOrParentWithAnyConfig,
            path: None,
        });

        File::create(temp_dir.path().join("lib/config_file.json")).unwrap();

        let targets_results = vec![
            (target_files("lib/hello.rb"), temp_dir.path().join("lib")),
            (target_files("greetings.rb"), temp_dir.path().to_path_buf()),
            (
                target_files("lib/tasks/some.rb"),
                temp_dir.path().join("lib"),
            ),
        ];

        for (target, result) in targets_results {
            let invocation_directory = invocation_directory_planner.compute(&target).unwrap();
            assert_eq!(invocation_directory, result);
        }
    }

    #[test]
    fn test_compute_target_directory_invocation_directory() {
        let (temp_dir, invocation_directory_planner) = setup(InvocationDirectoryDef {
            kind: InvocationDirectoryType::TargetDirectory,
            path: None,
        });

        let targets_results = vec![
            (target_files("lib/hello.rb"), temp_dir.path().join("lib")),
            (target_files("greetings.rb"), temp_dir.path().to_path_buf()),
            (
                target_files("lib/tasks/some.rb"),
                temp_dir.path().join("lib/tasks"),
            ),
        ];

        for (target, result) in targets_results {
            let invocation_directory = invocation_directory_planner.compute(&target).unwrap();
            assert_eq!(invocation_directory, result);
        }
    }

    #[test]
    fn test_compute_tool_directory_invocation_directory() {
        let (temp_dir, invocation_directory_planner) = setup(InvocationDirectoryDef {
            kind: InvocationDirectoryType::ToolDir,
            path: None,
        });
        reroute_tools_root(&temp_dir, &*invocation_directory_planner.tool);

        let tool_directory = temp_dir
            .path()
            .join(invocation_directory_planner.tool.directory());

        let targets_results = vec![
            (target_files("lib/hello.rb"), tool_directory.clone()),
            (target_files("greetings.rb"), tool_directory.clone()),
            (target_files("lib/tasks/some.rb"), tool_directory),
        ];

        for (target, result) in targets_results {
            let invocation_directory = invocation_directory_planner.compute(&target).unwrap();
            assert_eq!(invocation_directory, result);
        }
    }
}