use clap::Parser;
use git_progress_sync::{Cli, Config, LoadConfigError, exit_with_error, print_step};

fn main() {
	let cli = Cli::parse();

	let config_filepath = Config::get_default_config_filepath().unwrap_or_else(|| {
		exit_with_error("Failed to get default config filepath");
	});

	let config = Config::load(&config_filepath).unwrap_or_else(|e| match e {
		LoadConfigError::FileNotFound => {
			let default_config = Config::load_with_current_repo().unwrap_or_else(|| {
				exit_with_error("Failed to create default config");
			});

			default_config.save(&config_filepath).unwrap_or_else(|e| {
				exit_with_error(format!("Failed to save default config: {e}"));
			});

			default_config
		}
		_ => {
			exit_with_error(format!("Failed to load config: {e}"));
		}
	});

	if let Err(e) = cli.run(config_filepath, config) {
		exit_with_error(format!("git progress sync failed:\n{e}"));
	}

	print_step("Finished", "");
}
