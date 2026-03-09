use crate::GitProgressSyncError;
use git2::{ApplyLocation, Diff, DiffFormat, DiffOptions, Repository, StashFlags};
use std::fs;
use std::io::{BufWriter, Write};
use std::path::Path;

pub fn save_changes_to_file(
	repo: &Repository,
	output_path: impl AsRef<Path>,
) -> Result<(), GitProgressSyncError> {
	let head_commit = repo.head()?.peel_to_commit()?;
	let head_tree = head_commit.tree()?;

	let mut opts = DiffOptions::new();
	opts.include_untracked(true)
		.recurse_untracked_dirs(true)
		.show_untracked_content(true)
		.ignore_submodules(false);

	let diff = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))?;

	let patch_file =
		fs::File::create(output_path.as_ref()).map_err(GitProgressSyncError::SaveStashfile)?;
	let mut path_file_writer = BufWriter::new(patch_file);

	diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
		let origin = line.origin();
		if matches!(origin, '+' | '-' | ' ') && path_file_writer.write_all(&[origin as u8]).is_err()
		{
			return false;
		}
		path_file_writer.write_all(line.content()).is_ok()
	})
	.map_err(|e| e.into())
}

pub fn stash_changes(repo: &mut Repository, stash_name: &str) -> Result<git2::Oid, git2::Error> {
	let signature = repo.signature()?;

	let oid = repo.stash_save(&signature, stash_name, Some(StashFlags::INCLUDE_UNTRACKED))?;

	Ok(oid)
}

fn find_stash_index(
	repo: &mut Repository,
	stash_oid: &git2::Oid,
) -> Result<usize, GitProgressSyncError> {
	let mut stash_index = None;

	repo.stash_foreach(|index, _name, oid| -> bool {
		let matches = oid == stash_oid;
		if matches {
			stash_index = Some(index);
		}
		!matches
	})?;

	stash_index.ok_or(GitProgressSyncError::FailedToFindStash {
		stash_oid: *stash_oid,
	})
}

pub fn apply_stash(
	repo: &mut Repository,
	stash_oid: &git2::Oid,
) -> Result<(), GitProgressSyncError> {
	let stash_index = find_stash_index(repo, stash_oid)?;
	repo.stash_pop(stash_index, None).map_err(|e| e.into())
}

pub fn drop_stash(
	repo: &mut Repository,
	stash_oid: &git2::Oid,
) -> Result<(), GitProgressSyncError> {
	let stash_index = find_stash_index(repo, stash_oid)?;
	repo.stash_drop(stash_index).map_err(|e| e.into())
}

pub fn load_changes_from_file(
	repo: &mut Repository,
	pathfile_path: impl AsRef<Path>,
) -> Result<(), GitProgressSyncError> {
	let patch_bytes =
		fs::read(pathfile_path.as_ref()).map_err(GitProgressSyncError::ReadStashfile)?;

	let diff = Diff::from_buffer(&patch_bytes)?;

	repo.apply(&diff, ApplyLocation::WorkDir, None)
		.map_err(|e| e.into())
}

fn try_extract_repo_name_from_git_origin_remote(repo: &Repository) -> Option<String> {
	let origin_remote = repo.find_remote("origin").ok()?;
	let url = origin_remote.url()?;

	url.trim_end_matches(".git")
		.rsplit_once('/')
		.map(|(_, repo_name_part)| repo_name_part.to_string())
}

/// tries to get the 'origin' remote name for the repo by parsing the remote url
/// if there is no 'origin' remote or we cannot parse the remote url, we fallback to the directory
/// name of the repo
pub fn get_git_repo_name(repo: &Repository) -> Result<String, GitProgressSyncError> {
	match try_extract_repo_name_from_git_origin_remote(repo) {
		Some(repo_name) => Ok(repo_name),
		None => repo
			.workdir()
			.ok_or(GitProgressSyncError::FailedToDetermineRepoName)
			.and_then(|workdir| {
				workdir
					.file_name()
					.and_then(|os_str| os_str.to_str().map(str::to_string))
					.ok_or(GitProgressSyncError::FailedToDetermineRepoName)
			}),
	}
}

pub fn get_git_current_branch_name(repo: &Repository) -> Result<String, GitProgressSyncError> {
	let head = repo.head()?;

	let name = head
		.shorthand()
		.ok_or_else(|| std::io::Error::other("failed to get branch name"))?;

	Ok(name.to_string())
}
