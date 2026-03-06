use clap::{Parser, Subcommand};
use colored::Colorize;
use git2::Repository;
use std::{path::PathBuf, process::exit};

use crate::config::Config;
use crate::error::GitProgressSyncError;
use crate::git::{
	apply_stash, drop_stash, get_git_current_branch_name, get_git_current_repo_name,
	load_changes_from_file, save_changes_to_file,
};
use crate::stash_changes;

pub fn print_step(step: impl AsRef<str>, msg: impl AsRef<str>) {
	println!("{} {}", step.as_ref().bright_green().bold(), msg.as_ref());
}
pub fn print_error(error: impl AsRef<str>) {
	eprintln!("{}", error.as_ref().red().bold());
}
pub fn exit_with_error(error: impl AsRef<str>) -> ! {
	print_error(error);
	exit(1)
}

#[derive(Parser)]
pub struct Cli {
	#[command(subcommand)]
	subcommand: Option<CliSubcommand>,
}

impl Cli {
	pub fn run(self, config_filepath: PathBuf, config: Config) -> Result<(), GitProgressSyncError> {
		let mut repo = Repository::discover(".")?;

		let repo_name = get_git_current_repo_name(&repo)?;

		let branch_name = get_git_current_branch_name(&repo)?;

		let stash_filepath = config.get_stash_filepath(repo_name, branch_name);

		self.subcommand
			.unwrap_or_default()
			.run(config_filepath, stash_filepath, &mut repo)
	}
}

#[derive(Debug, Default, Clone, Subcommand)]
enum CliSubcommand {
	/// (default) loads changes from a stash file in the root directory
	#[default]
	Load,
	/// saves current changes to a stash file in the root directory
	Save,
	/// configures the root directory in the config.toml file
	Configure {
		#[arg(long)]
		root_directory: PathBuf,
	},
}

impl CliSubcommand {
	fn run(
		&self,
		config_filepath: PathBuf,
		stash_filepath: PathBuf,
		repo: &mut Repository,
	) -> Result<(), GitProgressSyncError> {
		match self {
			Self::Load => {
				print_step("Removing", "previous changes...");
				let tmp_stash_oid = match stash_changes(repo, TMP_GIT_PROGRESS_SYNC_STASH_NAME) {
					Ok(oid) => Some(oid),
					// returns code=NotFound when there are no previous changes
					Err(ref e) if e.code() == git2::ErrorCode::NotFound => None,
					Err(e) => return Err(e.into()),
				};

				print_step("Applying", "new changes...");
				match load_changes_from_file(repo, &stash_filepath) {
					Ok(()) => {
						if let Some(tmp_stash_oid) = tmp_stash_oid {
							print_step("Removing", "stashed previous changes...");

							drop_stash(repo, &tmp_stash_oid)?;
						}
						Ok(())
					}
					Err(e) => match tmp_stash_oid {
						Some(tmp_stash_oid) => {
							print_error(format!(
								"failed to load newest changes: {e}\nrestoring previous changes..."
							));

							apply_stash(repo, &tmp_stash_oid)
						}
						None => return Err(e),
					},
				}
			}
			CliSubcommand::Save => {
				print_step("Saving", "changes...");
				if let Some(stash_parent_directory) = stash_filepath.parent() {
					std::fs::create_dir_all(stash_parent_directory)?;
				}
				save_changes_to_file(repo, &stash_filepath)
			}
			CliSubcommand::Configure { root_directory } => {
				let new_config = Config::new(root_directory.to_path_buf());
				new_config.save(&config_filepath)?;
				Ok(())
			}
		}
	}
}

pub const TMP_GIT_PROGRESS_SYNC_STASH_NAME: &str =
	"previous changes before git_progress_sync (temporary)";
