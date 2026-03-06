use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LoadConfigError {
	#[error("config file not found")]
	FileNotFound,
	#[error("IO error: {0}")]
	IOError(#[from] std::io::Error),
	#[error("Toml error: {0}")]
	TomlError(#[from] toml::de::Error),
}

#[derive(Debug, Error)]
pub enum SaveConfigError {
	#[error("IO error: {0}")]
	IOError(#[from] std::io::Error),
	#[error("Toml error: {0}")]
	TomlError(#[from] toml::ser::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
	pub root_directory: PathBuf,
}

impl Config {
	pub fn new(root_directory: PathBuf) -> Self {
		Config { root_directory }
	}

	pub fn load_with_current_repo() -> Option<Self> {
		Self::get_default_root_directory().map(|root_directory| Self { root_directory })
	}

	pub fn load(config_filepath: impl AsRef<Path>) -> Result<Self, LoadConfigError> {
		let toml = std::fs::read_to_string(config_filepath).map_err(|e| {
			if e.kind() == std::io::ErrorKind::NotFound {
				LoadConfigError::FileNotFound
			} else {
				LoadConfigError::IOError(e)
			}
		})?;
		let config: Config = toml::from_str(&toml)?;
		Ok(config)
	}

	pub fn save(&self, config_filepath: &Path) -> Result<(), SaveConfigError> {
		let toml = toml::to_string(&self)?;
		if let Some(parent_directory) = config_filepath.parent() {
			std::fs::create_dir_all(parent_directory)?;
		}
		std::fs::write(config_filepath, toml)?;
		Ok(())
	}

	fn get_project_dirs() -> Option<directories::ProjectDirs> {
		directories::ProjectDirs::from("", "", "git_progress_sync")
	}

	pub fn get_default_config_filepath() -> Option<PathBuf> {
		Some(Self::get_project_dirs()?.config_dir().join("config.toml"))
	}

	fn get_default_root_directory() -> Option<PathBuf> {
		Some(Self::get_project_dirs()?.data_local_dir().join("stashes"))
	}

	/// returns the stash filepath of this current system/device
	pub fn get_stash_filepath_for_current_system(
		&self,
		repo_name: impl AsRef<str>,
		branch_name: impl AsRef<str>,
	) -> PathBuf {
		let repo_dir = self.root_directory.join(repo_name.as_ref());
		let branch_dir = repo_dir.join(branch_name.as_ref());
		let username = whoami::username().expect("failed to get username");
		let hostname = whoami::hostname().expect("failed to get hostname");
		let system_identifier = format!("{username}@{hostname}");
		branch_dir.join(format!("{}.stash", system_identifier))
	}

	/// returns all available stashes (filename without extension (filestem) are username@hostname)
	pub fn get_all_stash_filepaths(
		&self,
		repo_name: impl AsRef<str>,
		branch_name: impl AsRef<str>,
	) -> Vec<PathBuf> {
		let repo_dir = self.root_directory.join(repo_name.as_ref());
		let branch_dir = repo_dir.join(branch_name.as_ref());
		std::fs::read_dir(branch_dir)
			.map(|entries| {
				entries
					.into_iter()
					.filter_map(|e| e.ok().map(|e| e.path().to_path_buf()))
					.collect::<Vec<_>>()
			})
			.unwrap_or_default()
	}
}
