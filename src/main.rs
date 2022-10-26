#![warn(clippy::all)]
#![warn(clippy::unwrap_used)]

use clap::ArgMatches;
use clean::CargoClean;
pub use error::{Error, Result};
use flexi_logger::LogSpecification;
use log::info;
use miette::Context;
pub use miette::Result as MResult;

pub mod arg;
pub mod clean;
mod error;

type AResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const _: &str = include_str!("../Cargo.toml");

#[tokio::main]
async fn main() -> AResult<()> {
	#[cfg(debug_assertions)]
	let logdefault = "DEBUG";
	#[cfg(not(debug_assertions))]
	let logdefault = "INFO";
	flexi_logger::Logger::with(LogSpecification::env_or_parse(logdefault)?).start()?;
	let m: ArgMatches = self::arg::matches();
	let dry: bool = *m.get_one("dry").ok_or(Error::GetArgFailed("dry"))?;
	let mut cleaner = CargoClean::try_from(&m)?;
	cleaner.get_paths(false).await;
	let size_before = cleaner.get_size().await;
	if dry {
		if let Some(size) = size_before {
			info!("Size: {}", size.1);
		}
		info!("Stopping because of --dry");
		return Ok(());
	}
	cleaner.run().await;
	// Get Size again
	cleaner.get_paths(true).await;
	if let Some(s_before) = size_before {
		let size_after = cleaner
			.get_size()
			.await
			.ok_or(Error::SizeNotCalculated)
			.context("Size After not calculated")?;
		let s_before_i = s_before.0 as i64;
		let difference: i64 = s_before_i - size_after.0 as i64;
		let difference_str: String = {
			let mut out = human_bytes::human_bytes(difference as f64);
			if difference > 0 {
				out = format!("-{}", out);
			}
			out
		};
		let percent = (100.0 / s_before_i as f64) * (difference as f64);
		info!(
			"{} => {} ({}: {}{:.3} %)",
			s_before.1,
			size_after.1,
			difference_str,
			if percent > 0.0 { "-" } else { "" },
			percent
		);
	}
	Ok(())
}
