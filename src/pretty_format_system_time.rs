use std::{str::FromStr, time::SystemTime};

use chrono::{Local, Locale, TimeDelta};

/// almost criminal there is not a good general crossplatform crate for this :<
fn try_get_time_locale() -> Option<String> {
	std::env::var("LC_TIME")
		.ok()
		.and_then(|str| str.split('.').next().map(str::to_string))
		.or(std::env::var("LC_ALL")
			.ok()
			.and_then(|str| str.split('.').next().map(str::to_string)))
		.or(sys_locale::get_locale())
}

/// format system time with system locale
pub fn pretty_format_system_time(system_time: SystemTime) -> String {
	let diff = system_time
		.elapsed()
		.expect("provided system time should not be from the future");

	let diff = TimeDelta::from_std(diff).unwrap();
	let local_now = Local::now();
	let local_time = local_now.checked_sub_signed(diff).unwrap();

	let diff_mins = diff.num_minutes();
	if diff_mins < 60 {
		format!("{} mins ago", diff_mins)
	} else {
		let diff_days = diff.num_days();
		let system_locale = try_get_time_locale()
			.and_then(|str| Locale::from_str(&str).ok())
			.unwrap_or_default();

		match diff_days {
			0 => format!("{} today", local_time.format_localized("%X", system_locale)),
			1 => format!(
				"{} yesterday",
				local_time.format_localized("%X", system_locale)
			),
			_ => local_time
				.format_localized("%x %X", system_locale)
				.to_string(),
		}
	}
}
