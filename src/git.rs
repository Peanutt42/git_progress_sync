use crate::{GitProgressSyncError, RunGitError, RunGitErrorKind, StdErr};
use std::{fs::File, io::Read, path::Path};

pub fn git_stash() -> Result<(), RunGitError> {
	run_git_command([
		"stash".to_string(),
		"push".to_string(),
		"-k".to_string(),
		"-u".to_string(),
		"-m".to_string(),
		"git_progress_sync stash (temporary)".to_string(),
	])
}

pub fn run_git_command(args: impl Into<Vec<String>>) -> Result<(), RunGitError> {
	let args = args.into();

	let output = std::process::Command::new("git")
		.args(&args)
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

pub fn save_stash(output_filepath: impl AsRef<Path>) -> Result<(), GitProgressSyncError> {
	let args = vec![
		"stash".to_string(),
		"show".to_string(),
		"--binary".to_string(),
		"-u".to_string(),
	];

	let output_file = File::create(output_filepath).map_err(GitProgressSyncError::SaveFile)?;

	let mut child = std::process::Command::new("git")
		.args(&args)
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
