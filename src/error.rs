use crate::{LoadConfigError, SaveConfigError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitProgressSyncError {
	#[error(transparent)]
	Git(#[from] git2::Error),
	#[error("Stdio error: {0}")]
	Stdio(#[from] std::io::Error),
	#[error("Failed to save stash file to {0}")]
	SaveStashfile(std::io::Error),
	#[error("Failed to read stash file at {0}")]
	ReadStashfile(std::io::Error),
	#[error("Failed to find the stash with oid {stash_oid}")]
	FailedToFindStash { stash_oid: git2::Oid },
	#[error(transparent)]
	SaveConfig(#[from] SaveConfigError),
	#[error(transparent)]
	LoadConfig(#[from] LoadConfigError),
}
