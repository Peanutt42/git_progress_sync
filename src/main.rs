use clap::Parser;
use std::{path::PathBuf, process::exit};

mod error;
use error::{GitProgressSyncError, RunGitError, RunGitErrorKind, StdErr};

mod git;
use git::{git_stash, run_git_command, save_stash};

fn main() {
	let cli = Cli::parse();
	if let Err(e) = cli.run() {
		eprintln!("git progress sync failed:\n{e}");
		exit(1);
	}
}

#[derive(Parser)]
struct Cli {
	#[arg(short, long, default_value_t = false)]
	save: bool,

	#[arg(long, required = true)]
	stash_filepath: PathBuf,
}

impl Cli {
	fn run(self) -> Result<(), GitProgressSyncError> {
		if self.save {
			println!("Collecting changes...");
			git_stash()?;

			println!("Saving changes...");
			if let Some(stash_parent_directory) = self.stash_filepath.parent() {
				std::fs::create_dir_all(stash_parent_directory)?;
			}
			save_stash(self.stash_filepath)?;

			println!("Restoring changes...");
			let pop_result = run_git_command(["stash".to_string(), "pop".to_string()]);
			match pop_result {
				Err(RunGitError {
					kind: RunGitErrorKind::NonZeroExitCode { exit_code: 1, .. },
					..
				}) => {
					// no changes to pop
				}
				_ => pop_result?,
			}
		} else {
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
				"--allow-empty".to_string(),
				self.stash_filepath.to_string_lossy().into_owned(),
			])?;
		}
		Ok(())
	}
}
