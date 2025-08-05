use crate::{GitProgressSyncError, RunGitError, RunGitErrorKind, StdErr};
use std::{
	fs::File,
	io::Read,
	path::{Path, PathBuf},
};

pub fn git_stash(working_directory: impl AsRef<Path>) -> Result<(), RunGitError> {
	run_git_command(
		[
			"stash".to_string(),
			"push".to_string(),
			"-k".to_string(),
			"-u".to_string(),
			"-m".to_string(),
			"git_progress_sync stash (temporary)".to_string(),
		],
		working_directory,
	)
}

pub fn run_git_command(
	args: impl Into<Vec<String>>,
	working_directory: impl AsRef<Path>,
) -> Result<(), RunGitError> {
	let args = args.into();

	let output = std::process::Command::new("git")
		.args(&args)
		.current_dir(working_directory)
		.output()
		.map_err(|e| RunGitError {
			args: args.clone(),
			kind: RunGitErrorKind::StdioError(e),
		})?;
	if output.status.success() {
		Ok(())
	} else {
		Err(RunGitError {
			args,
			kind: RunGitErrorKind::NonZeroExitCode {
				exit_code: output.status.code().unwrap_or(-1),
				stderr: StdErr::new(output.stderr),
			},
		})
	}
}

pub fn save_stash(
	output_filepath: impl AsRef<Path>,
	working_directory: impl AsRef<Path>,
) -> Result<(), GitProgressSyncError> {
	let args = vec![
		"stash".to_string(),
		"show".to_string(),
		"--binary".to_string(),
		"-u".to_string(),
	];

	let output_file = File::create(output_filepath).map_err(GitProgressSyncError::SaveFile)?;

	let mut child = std::process::Command::new("git")
		.args(&args)
		.current_dir(working_directory)
		.stdout(output_file)
		.stderr(std::process::Stdio::piped())
		.spawn()
		.map_err(GitProgressSyncError::Stdio)?;

	let exit_status = child.wait()?;
	if exit_status.success() {
		Ok(())
	} else {
		let mut stderr_output: Vec<u8> = Vec::new();
		if let Some(mut stderr) = child.stderr.take() {
			stderr.read_to_end(&mut stderr_output)?;
		}

		Err(RunGitError {
			args,
			kind: RunGitErrorKind::NonZeroExitCode {
				exit_code: exit_status.code().unwrap_or(-1),
				stderr: StdErr::new(stderr_output),
			},
		}
		.into())
	}
}

/// by repo name, the name of the directory containing the .git directory is meant
pub fn get_git_current_repo_name() -> Result<String, RunGitError> {
	match get_git_current_repo_root_directory()?.file_name() {
		Some(filename) => Ok(filename.to_string_lossy().to_string()),
		None => Err(RunGitError {
			args: vec![],
			kind: RunGitErrorKind::StdioError(std::io::Error::other(
				"failed to get root git repo directory filename",
			)),
		}),
	}
}

pub fn get_git_current_repo_root_directory() -> Result<PathBuf, RunGitError> {
	let args = vec!["rev-parse".to_string(), "--show-toplevel".to_string()];

	let output = std::process::Command::new("git")
		.args(&args)
		.output()
		.map_err(|e| RunGitError {
			args: args.clone(),
			kind: RunGitErrorKind::StdioError(e),
		})?;

	if output.status.success() {
		Ok(PathBuf::from(
			String::from_utf8_lossy(&output.stdout).trim().to_string(),
		))
	} else {
		Err(RunGitError {
			args,
			kind: RunGitErrorKind::NonZeroExitCode {
				exit_code: output.status.code().unwrap_or(-1),
				stderr: StdErr::new(output.stderr),
			},
		})
	}
}

pub fn get_git_current_branch_name() -> Result<String, RunGitError> {
	let args = vec![
		"rev-parse".to_string(),
		"--abbrev-ref".to_string(),
		"HEAD".to_string(),
	];

	let output = std::process::Command::new("git")
		.args(&args)
		.output()
		.map_err(|e| RunGitError {
			args: args.clone(),
			kind: RunGitErrorKind::StdioError(e),
		})?;

	if output.status.success() {
		Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
	} else {
		Err(RunGitError {
			args,
			kind: RunGitErrorKind::NonZeroExitCode {
				exit_code: output.status.code().unwrap_or(-1),
				stderr: StdErr::new(output.stderr),
			},
		})
	}
}
