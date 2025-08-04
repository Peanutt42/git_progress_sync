use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::exit};

mod error;
use error::{GitProgressSyncError, RunGitError, RunGitErrorKind, StdErr};

mod git;
use git::{get_git_current_branch_name, git_stash, run_git_command, save_stash};

mod config;
use config::{Config, LoadConfigError, SaveConfigError};

fn main() {
	let cli = Cli::parse();

	let config_filepath = Config::get_default_config_filepath().unwrap_or_else(|| {
		eprintln!("Failed to get default config filepath");
		exit(1);
	});

	let config = Config::load(&config_filepath).unwrap_or_else(|e| match e {
		LoadConfigError::FileNotFound => {
			let default_config = Config::default().unwrap_or_else(|| {
				eprintln!("Failed to create default config");
				exit(1);
			});

			default_config.save(&config_filepath).unwrap_or_else(|e| {
				eprintln!("Failed to save default config: {e}");
				exit(1);
			});

			default_config
		}
		_ => {
			eprintln!("Failed to load config: {e}");
			exit(1);
		}
	});

	if let Err(e) = cli.run(config_filepath, config) {
		eprintln!("git progress sync failed:\n{e}");
		exit(1);
	}
}

#[derive(Parser)]
struct Cli {
	#[command(subcommand)]
	subcommand: Option<CliSubcommand>,
}

impl Cli {
	fn run(self, config_filepath: PathBuf, config: Config) -> Result<(), GitProgressSyncError> {
		let repo_name = std::env::current_dir()
			.expect("could not get the current working directory")
			.file_name()
			.expect("failed to get the filename of the current working directory")
			.to_string_lossy()
			.to_string();

		let branch_name = get_git_current_branch_name()?;

		let stash_filepath = config.get_stash_filepath(repo_name, branch_name);

		self.subcommand
			.unwrap_or_default()
			.run(config_filepath, stash_filepath)
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
	) -> Result<(), GitProgressSyncError> {
		match self {
			Self::Load => {
				println!("Removing old changes...");
				git_stash()?;
				let drop_result = run_git_command(["stash".to_string(), "drop".to_string()]);
				match drop_result {
					Err(RunGitError {
						kind: RunGitErrorKind::NonZeroExitCode { exit_code: 1, .. },
						..
					}) => {
						// no stash to drop, because there were no uncommitted changes
					}
					_ => {
						// return all other errors
						drop_result?;
					}
				}

				println!("Applying new changes...");
				run_git_command([
					"apply".to_string(),
					"--binary".to_string(),
					stash_filepath.to_string_lossy().into_owned(),
				])
				.map_err(|e| e.into())
			}
			CliSubcommand::Save => {
				println!("Collecting changes...");
				git_stash()?;

				println!("Saving changes...");
				if let Some(stash_parent_directory) = stash_filepath.parent() {
					std::fs::create_dir_all(stash_parent_directory)?;
				}
				save_stash(stash_filepath)?;

				println!("Restoring changes...");
				let pop_result = run_git_command(["stash".to_string(), "pop".to_string()]);
				match pop_result {
					Err(RunGitError {
						kind: RunGitErrorKind::NonZeroExitCode { exit_code: 1, .. },
						..
					}) => {
						Ok(()) // no changes to pop
					}
					_ => pop_result.map_err(|e| e.into()),
				}
			}
			CliSubcommand::Configure { root_directory } => {
				let new_config = Config::new(root_directory.to_path_buf());
				new_config.save(&config_filepath)?;
				Ok(())
			}
		}
	}
}
