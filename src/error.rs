use crate::{LoadConfigError, SaveConfigError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitProgressSyncError {
	#[error(transparent)]
	RunGit(#[from] RunGitError),
	#[error("Stdio error: {0}")]
	Stdio(#[from] std::io::Error),
	#[error("Failed to save stash to file: {0}")]
	SaveFile(std::io::Error),
	#[error(transparent)]
	SaveConfig(#[from] SaveConfigError),
	#[error(transparent)]
	LoadConfig(#[from] LoadConfigError),
}

#[derive(Debug, Error)]
#[error("Failed to run 'git {formatted_args}': {kind}", formatted_args = .args.join(" "))]
pub struct RunGitError {
	pub args: Vec<String>,
	#[source]
	pub kind: RunGitErrorKind,
}

#[derive(Debug, Error)]
pub enum RunGitErrorKind {
	#[error(transparent)]
	StdioError(#[from] std::io::Error),
	#[error("non zero exit code {exit_code}:\nstderr:\n{stderr}")]
	NonZeroExitCode { exit_code: i32, stderr: StdErr },
}

#[derive(Debug)]
pub struct StdErr(Vec<u8>);

impl StdErr {
	pub fn new(stderr: Vec<u8>) -> Self {
		Self(stderr)
	}
}

impl std::fmt::Display for StdErr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", String::from_utf8_lossy(&self.0))
	}
}
