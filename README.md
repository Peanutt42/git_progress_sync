# Git Progress Sync

Syncs your changed files in your git repo to your self hosted server to easily continue where you left off.

Most useful for switching between computers whenever you want.

For now, saves to a local file, that could be OneDrive using rclone.

## Installation

`git_progress_sync` is available as a overlay for NixOS:
```nix
{
  inputs = {
    git_progress_sync.url = "github:Peanutt42/git_progress_sync";
  };

  # overlay is git_progress_sync.overlays.default
}
```

To build, you need some system packages: the openssl dev package and the pkg-config package
<br>
If you use NixOS, you can just run `nix develop`
<br>
To install, just run:

```bash
cargo install --path .
```

## Configuration

```bash
git_progress_sync configure --root-directory /path/to/root/directory
```

this will configure the config file in `~/.config/git-progress-sync/config.toml`:

```toml
root_directory = "/path/to/root/directory"
```

## Usage

To save your changes:

```bash
git_progress_sync save
```

To load your changes:

```bash
git_progress_sync
```

this will do the same as this:

```bash
git_progress_sync load
```
