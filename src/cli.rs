use crate::config::Config;
use crate::error::GitProgressSyncError;
use crate::git::{
	apply_stash, drop_stash, get_git_current_branch_name, get_git_repo_name,
	load_changes_from_file, save_changes_to_file,
};
use crate::{pretty_format_system_time, stash_changes};
use clap::{Parser, Subcommand};
use colored::Colorize;
use git2::Repository;
use inquire::ui::RenderConfig;
use std::time::SystemTime;
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
#[command(version)]
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
	/// lists all available stashes in the root directory
	List,
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
		let repo_name = get_git_repo_name(repo)?;

		let branch_name = get_git_current_branch_name(repo)?;

		match self {
			Self::Load => {
				print_step("Collecting", "all available stashes...");
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
					stash_filepath.clone()
				} else {
					match Self::choose_stash_filepath(&stash_filepaths) {
						Some(stash_filepath) => stash_filepath,
						None => return Ok(()),
					}
				};

				print_step("Removing", "previous local changes...");
				let tmp_stash_oid = match stash_changes(repo, TMP_GIT_PROGRESS_SYNC_STASH_NAME) {
					Ok(oid) => Some(oid),
					// returns code=NotFound when there are no previous changes
					Err(ref e) if e.code() == git2::ErrorCode::NotFound => None,
					Err(e) => return Err(e.into()),
				};

				print_step(
					"Applying",
					format!(
						"stash from {}...",
						stash_filepath
							.file_stem()
							.and_then(std::ffi::OsStr::to_str)
							.unwrap_or_default()
							.cyan()
					),
				);

				match load_changes_from_file(repo, stash_filepath) {
					Ok(()) => {
						if let Some(tmp_stash_oid) = tmp_stash_oid {
							print_step("Removing", "stashed previous local changes...");

							drop_stash(repo, &tmp_stash_oid)?;
						}
						Ok(())
					}
					Err(e) => match tmp_stash_oid {
						Some(tmp_stash_oid) => {
							print_error(format!(
								"failed to load newest changes: {e}\nrestoring previous local changes..."
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
			CliSubcommand::List => {
				let stash_filepaths = config.get_all_stash_filepaths(&repo_name, &branch_name);
				if stash_filepaths.is_empty() {
					println!(
						"there are no stashes available for branch {branch_name} in repo {repo_name}"
					);
				} else {
					let options = create_stash_filepath_options(&stash_filepaths);

					for option in options {
						if option.is_current_device {
							println!(
								"  {}\t{}\t{}",
								option.identifier.cyan(),
								option.last_modified_formatted,
								"(this device)".green()
							);
						} else {
							println!(
								"  {}\t{}",
								option.identifier.cyan(),
								option.last_modified_formatted
							);
						}
					}
				}
				Ok(())
			}
			CliSubcommand::Configure { root_directory } => {
				let new_config = Config::new(root_directory.to_path_buf());
				new_config.save(&config_filepath)?;
				Ok(())
			}
		}
	}

	fn choose_stash_filepath(stash_filepaths: &[PathBuf]) -> Option<PathBuf> {
		let options = create_stash_filepath_options(stash_filepaths);

		let invisible_prompt_prefix = inquire::ui::Styled::new("");
		let render_config = RenderConfig::default().with_prompt_prefix(invisible_prompt_prefix);

		loop {
			match inquire::Select::new(
				"There are multiple stashes from different devices:",
				options.clone(),
			)
			.with_help_message(
				"↑↓ to move, enter to select, type to filter, or press Esc to cancel",
			)
			.with_render_config(render_config)
			.raw_prompt_skippable()
			{
				Ok(selected_option) => {
					return selected_option.map(|o| o.value.filepath);
				}
				Err(e) => {
					print_error(format!("something went wrong: {e}, please try again"));
				}
			}
		}
	}
}

#[derive(Clone)]
struct StashFilepathOption {
	filepath: PathBuf,
	identifier: String,
	is_current_device: bool,
	last_modified: SystemTime,
	last_modified_formatted: String,
}
impl std::fmt::Display for StashFilepathOption {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}\t{}", self.identifier, self.last_modified_formatted)?;
		if self.is_current_device {
			write!(f, "\t{}", "(this device)".green())
		} else {
			Ok(())
		}
	}
}

fn create_stash_filepath_options(stash_filepaths: &[PathBuf]) -> Vec<StashFilepathOption> {
	let current_system_identifier = Config::get_current_system_identifier();

	let get_filepath_filestem = |p: &PathBuf| -> Option<String> {
		p.file_stem().and_then(|s| s.to_str()).map(str::to_string)
	};

	let mut options = stash_filepaths
		.iter()
		.filter_map(|filepath| {
			let identifier = get_filepath_filestem(filepath)?;
			let is_current_device = identifier == current_system_identifier;
			let last_modified = filepath.metadata().ok().and_then(|m| m.modified().ok())?;
			let last_modified_formatted = pretty_format_system_time(last_modified);

			Some(StashFilepathOption {
				filepath: filepath.clone(),
				identifier,
				is_current_device,
				last_modified,
				last_modified_formatted,
			})
		})
		.collect::<Vec<StashFilepathOption>>();

	// sorts options by latest to oldest
	options.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

	let last_option_index = options.len() - 1;

	if let Some(current_device_option_index) =
		options.iter().position(|options| options.is_current_device)
	{
		options.swap(current_device_option_index, last_option_index);
	}

	options
}

pub const TMP_GIT_PROGRESS_SYNC_STASH_NAME: &str =
	"previous changes before git_progress_sync (temporary)";
