use crate::config::Config;
use crate::error::GitProgressSyncError;
use crate::git::{
	apply_stash, drop_stash, get_git_current_branch_name, get_git_current_repo_name,
	load_changes_from_file, save_changes_to_file,
};
use crate::stash_changes;
use clap::{Parser, Subcommand};
use colored::Colorize;
use git2::Repository;
use std::{path::PathBuf, process::exit};

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

		self.subcommand
			.unwrap_or_default()
			.run(config, config_filepath, &mut repo)
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
		config: Config,
		config_filepath: PathBuf,
		repo: &mut Repository,
	) -> Result<(), GitProgressSyncError> {
		let repo_name = get_git_current_repo_name(repo)?;

		let branch_name = get_git_current_branch_name(repo)?;

		match self {
			Self::Load => {
				let stash_filepaths = config.get_all_stash_filepaths(&repo_name, &branch_name);
				if stash_filepaths.is_empty() {
					print_error(format!(
						"there are no stashes to load for branch {branch_name} in repo {repo_name}"
					));
					return Ok(());
				}

				let stash_filepath = if stash_filepaths.len() == 1
					&& let Some(stash_filepath) = stash_filepaths.first()
				{
					stash_filepath
				} else {
					match Self::choose_stash_filepath(&stash_filepaths)
						.and_then(|chosen_index| stash_filepaths.get(chosen_index))
					{
						Some(stash_filepath) => stash_filepath,
						None => return Ok(()),
					}
				};

				print_step("Removing", "previous changes...");
				let tmp_stash_oid = match stash_changes(repo, TMP_GIT_PROGRESS_SYNC_STASH_NAME) {
					Ok(oid) => Some(oid),
					// returns code=NotFound when there are no previous changes
					Err(ref e) if e.code() == git2::ErrorCode::NotFound => None,
					Err(e) => return Err(e.into()),
				};

				print_step("Applying", "new changes...");
				match load_changes_from_file(repo, stash_filepath) {
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
						None => Err(e),
					},
				}
			}
			CliSubcommand::Save => {
				let stash_filepath =
					config.get_stash_filepath_for_current_system(repo_name, branch_name);

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

	fn choose_stash_filepath(stash_filepaths: &[PathBuf]) -> Option<usize> {
		let options = stash_filepaths
			.iter()
			.filter_map(|f| f.file_stem().and_then(|f| f.to_str()).map(str::to_string))
			.collect::<Vec<String>>();

		loop {
			match inquire::Select::new(
				"There are multiple stashes from diverent devices. Please choose one or press Esc or Ctrl+C to cancel:",
				options.clone(),
			)
			.raw_prompt_skippable()
			{
				Ok(selected_option) => return selected_option.map(|o| o.index),
				Err(e) => {
					print_error(format!("something went wrong: {e}, please try again"));
				}
			}
		}
	}
}

pub const TMP_GIT_PROGRESS_SYNC_STASH_NAME: &str =
	"previous changes before git_progress_sync (temporary)";
