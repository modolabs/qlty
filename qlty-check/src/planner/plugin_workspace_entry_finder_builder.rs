use anyhow::{bail, Result};
use qlty_analysis::utils::fs::path_to_string;
use qlty_analysis::workspace_entries::TargetMode;
use qlty_analysis::{
    git::GitDiff, workspace_entries::AndMatcher, FileMatcher, GlobsMatcher, PrefixMatcher,
    WorkspaceEntryFinder, WorkspaceEntryMatcher, WorkspaceEntrySource,
};
use qlty_analysis::{AllSource, ArgsSource, DiffSource};
use qlty_config::config::issue_transformer::{IssueTransformer, NullIssueTransformer};
use qlty_config::config::Ignore;
use qlty_config::{FileType, Workspace};
use std::sync::Arc;
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Default, Clone)]
pub struct PluginWorkspaceEntryFinderBuilder {
    pub mode: TargetMode,
    pub root: PathBuf,
    pub paths: Vec<PathBuf>,
    pub file_types: HashMap<String, FileType>,
    pub ignores: Vec<Ignore>,
    pub cached_git_diff: Option<GitDiff>,
    pub prefix: Option<String>,
    pub source: Option<Arc<dyn WorkspaceEntrySource>>,
}

impl PluginWorkspaceEntryFinderBuilder {
    pub fn build(&mut self, language_names: &[String]) -> Result<WorkspaceEntryFinder> {
        Ok(WorkspaceEntryFinder::new(
            self.source()?,
            self.matcher(language_names)?,
        ))
    }

    pub fn diff_line_filter(&mut self) -> Result<Box<dyn IssueTransformer>> {
        match self.mode {
            TargetMode::HeadDiff | TargetMode::UpstreamDiff(_) => {
                Ok(Box::new(self.git_diff()?.line_filter))
            }
            _ => Ok(Box::new(NullIssueTransformer)),
        }
    }

    fn source(&mut self) -> Result<Arc<dyn WorkspaceEntrySource>> {
        if self.source.is_none() {
            self.source = Some(match self.mode {
                TargetMode::All | TargetMode::Sample(_) => {
                    Arc::new(AllSource::new(self.root.clone()))
                }
                TargetMode::Paths(_) => {
                    Arc::new(ArgsSource::new(self.root.clone(), self.paths.clone()))
                }
                TargetMode::UpstreamDiff(_) => {
                    Arc::new(DiffSource::new(self.git_diff()?.changed_files, &self.root))
                }
                TargetMode::HeadDiff => {
                    Arc::new(DiffSource::new(self.git_diff()?.changed_files, &self.root))
                }
            });
        }
        if let Some(source) = &self.source {
            Ok(source.clone())
        } else {
            bail!("Invalid workspace source")
        }
    }

    fn matcher(&self, language_names: &[String]) -> Result<Box<dyn WorkspaceEntryMatcher>> {
        let mut matchers: Vec<Box<dyn WorkspaceEntryMatcher>> = vec![];

        matchers.push(Box::new(FileMatcher));

        let file_types = self
            .file_types
            .iter()
            .filter_map(|(name, file_type)| {
                if language_names.contains(&name) {
                    Some(file_type.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let file_matcher = GlobsMatcher::new_for_file_types(&file_types)?;

        matchers.push(Box::new(file_matcher));

        matchers.push(Box::new(PrefixMatcher::new(
            path_to_string(Workspace::current_dir()),
            self.root.to_owned(),
        )));

        if let Some(prefix) = &self.prefix {
            matchers.push(Box::new(PrefixMatcher::new(
                path_to_string(self.root.join(prefix)),
                self.root.to_owned(),
            )));
        }

        let ignores = self
            .ignores
            .iter()
            .flat_map(|i| i.file_patterns.clone())
            .collect::<Vec<_>>();

        let ignores = GlobsMatcher::new_for_globs(&ignores, false)?;
        matchers.push(Box::new(ignores));

        Ok(Box::new(AndMatcher::new(matchers)))
    }

    fn git_diff(&mut self) -> Result<GitDiff> {
        if let Some(diff) = &self.cached_git_diff {
            return Ok(diff.clone());
        }

        let diff = self.compute_git_diff()?;
        self.cached_git_diff = Some(diff.clone());
        Ok(diff)
    }

    fn compute_git_diff(&self) -> Result<GitDiff> {
        let git_diff = GitDiff::compute(self.mode.diff_mode(), &self.root)?;
        Ok(git_diff)
    }
}
