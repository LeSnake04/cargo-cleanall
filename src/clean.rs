use crate::MResult;
use std::path::{Path, PathBuf};

use clap::{parser::ValuesRef, ArgMatches};
#[allow(unused_imports)]
use log::warn;
use log::{debug, error, trace};
use miette::{Context, IntoDiagnostic};
use path_absolutize::Absolutize;
use tokio::{spawn as tspawn, task::JoinHandle};
use unwrap_or::unwrap_ok_or;
use walkdir::WalkDir;

use crate::error::{Error, Result};

type Handles<T> = Vec<JoinHandle<T>>;
type SizeHandles = Handles<Option<u64>>;
// type RunResult = AResult<(std::process::Output, PathBuf)>;
type MRunResult = MResult<(std::process::Output, PathBuf)>;

#[derive(Debug)]
pub struct CargoClean {
	hidden: bool,
	workspaces: Vec<PathBuf>,
	roots: Vec<PathBuf>,
	size_handles: Option<SizeHandles>,
	ignore: Option<Vec<PathBuf>>,
}

impl TryFrom<&ArgMatches> for CargoClean {
	type Error = Error;
	fn try_from(m: &ArgMatches) -> Result<Self> {
		Ok(Self {
			hidden: *m.get_one("hidden").ok_or(Error::ArgNotSet("hidden"))?,
			roots: m
				.get_many("PATHS")
				.ok_or(Error::ArgNotSet("PATHS"))?
				.cloned()
				.filter_map(|p: PathBuf| {
					Some(
						unwrap_ok_or!(p.absolutize(), e, {
							error!("Error when absolitizing: {e}");
							return None;
						})
						.to_path_buf(),
					)
				})
				.collect(),
			workspaces: Vec::new(),
			size_handles: (!m
				.get_one::<bool>("no-size")
				.ok_or(Error::ArgNotSet("no-size"))?)
			.then_some(Vec::new()),
			ignore: m.get_many("ignore").map(|i: ValuesRef<PathBuf>| {
				i.map(|p: &PathBuf| {
					let mut out = p
						.absolutize()
						.unwrap_or_else(|_| panic!("failed to absolutize {}", p.display()))
						.to_path_buf();
					if out.is_file() {
						out = out
							.parent()
							.unwrap_or_else(|| panic!("Failed to get parent of {}", out.display()))
							.to_path_buf();
					}
					out
				})
				.collect()
			}),
		})
	}
}

#[derive(Debug)]
struct HandleTaskOut {
	pub paths: Vec<PathBuf>,
	pub size_handles: Option<SizeHandles>,
	pub tasks: Vec<PathBuf>,
}

impl HandleTaskOut {
	fn new(size: bool) -> HandleTaskOut {
		Self {
			size_handles: size.then_some(Vec::new()),
			paths: Vec::new(),
			tasks: Vec::new(),
		}
	}
}

impl CargoClean {
	#[inline]
	async fn get_path<'a>(&mut self, root: impl AsRef<Path>, size_only: bool) -> Result<()> {
		let mut tasks: Vec<PathBuf> = vec![root.as_ref().to_path_buf()];
		let mut tasks_handles: Handles<HandleTaskOut> = Vec::new();
		let mut init_run = true;
		let hidden: bool = self.hidden;
		let size: bool = self.size_handles.is_some();
		let ignore = self.ignore.as_mut().map(|i| {
			let mut out: Vec<PathBuf> = Vec::new();
			out.append(i);
			out
		});
		while !tasks_handles.is_empty() || init_run {
			init_run = false;
			while tasks.get(0).is_some() {
				tasks_handles.push(
					handle_task(
						tasks.swap_remove(0),
						size,
						hidden,
						size_only,
						ignore.clone(),
					)
					.await,
				);
			}
			let mut i = 0;
			while i < tasks_handles.len() {
				if let Some(t) = tasks_handles.get(i) {
					if t.is_finished() {
						let t = tasks_handles.remove(i);
						let mut out: HandleTaskOut = unwrap_ok_or!(t.await, e, {
							error!("{e}");
							break;
						});
						if !out.paths.is_empty() {
							self.workspaces.append(&mut out.paths);
						}
						if !out.tasks.is_empty() {
							tasks.append(&mut out.tasks)
						}
						if let Some(mut o_size) = out.size_handles {
							if !o_size.is_empty() {
								if let Some(s) = self.size_handles.as_mut() {
									s.append(&mut o_size)
								}
							}
						}
					} else {
						i += 1;
					}
				} else {
					break;
				}
			}
		}
		Ok(())
	}
	pub async fn get_paths(&mut self, size_only: bool) {
		trace!("Size only: {}", size_only);
		for r in self.roots.clone() {
			if let Err(e) = self.get_path(r, size_only).await {
				error!("{e}")
			}
		}
	}
	pub async fn get_size(&mut self) -> Option<(u64, String)> {
		let size_handles: &mut SizeHandles = match self.size_handles {
			Some(ref mut s) => s,
			None => return None,
		};
		let mut out: u64 = 0;
		let mut i = 0;
		while i < size_handles.len() {
			if let Some(s) = size_handles.get(i) {
				if s.is_finished() {
					let size_handle = size_handles.swap_remove(0);
					if let Ok(Some(size)) = size_handle.await {
						(size > 0).then(|| out += size);
					};
				} else {
					i += 1
				}
			} else {
				break;
			}
		}
		Some((out, human_bytes::human_bytes(out as f64)))
	}
	pub async fn run(&mut self) {
		let workspaces: Vec<PathBuf> = {
			let mut out = Vec::new();
			out.append(&mut self.workspaces);
			out
		};
		let mut tasks: Handles<MRunResult> = Vec::new();
		for p in workspaces {
			tasks.push(tspawn(async move {
				clear_workspace(p.clone())
					.context(format!("Failed to clear project {}", p.display()))
			}));
		}
		let mut i: usize;
		while !tasks.is_empty() {
			i = 0;
			while i < tasks.len() {
				if let Some(task) = tasks.get(i) {
					if task.is_finished() {
						let task = tasks.remove(i);
						match task.await {
							Ok(Ok(t)) => {
								// If getting parent fails, show path to Cargo.toml instead.
								debug!("Cleared project {}", t.1.parent().unwrap_or(&t.1).display())
							}
							Ok(Err(e)) => error!("{e:?}"),
							Err(e) => error!("{e}"),
						}
					} else {
						i += 1;
					}
				} else {
					break;
				}
			}
		}
	}
}
#[inline]
fn clear_workspace(manifest: PathBuf) -> MRunResult {
	use std::process::Command;
	// For some reason Cargo doesn't like a project being run and cleaned at the same time... .-.
	#[cfg(debug_assertions)]
	if manifest
		.parent()
		.ok_or_else(|| Error::ParentNotFound(manifest.clone()))?
		.file_name()
		.ok_or(Error::NoFileName)?
		== "cargo-cleanall"
	{
		warn!("Skipping cargo-cleanall");
		Err(Error::Skipped)?;
	};
	Ok((
		Command::new("cargo")
			.arg("clean")
			.args([
				"--manifest-path",
				manifest.to_str().ok_or(Error::ManifestToStrFailed)?,
			])
			.output()
			.into_diagnostic()?,
		manifest,
	))
}
async fn handle_task(
	task: PathBuf,
	size: bool,
	hidden: bool,
	size_only: bool,
	ignore: Option<Vec<PathBuf>>,
) -> JoinHandle<HandleTaskOut> {
	tspawn(async move {
		let mut out = HandleTaskOut::new(size);
		for p in WalkDir::new(task) {
			let p = unwrap_ok_or!(p, _, continue);
			let p_path: PathBuf = p.clone().into_path();
			trace!("Checking for {}", p_path.display());
			if !hidden
				&& p.file_name()
					.to_str()
					.map(|s| s.starts_with('.'))
					.unwrap_or(false)
			{
				continue;
			}
			if p.path().is_dir() {
				if let Some(ref i) = ignore {
					if i.contains(&p_path) {
						debug!("Skipped scaning {} because its ignored", p_path.display());
						continue;
					} else {
						out.tasks.push(p_path);
						continue;
					}
				}
			}
			if !size_only && p.file_name() == "Cargo.toml" {
				debug!(
					"Found Project: {}",
					p.path().parent().unwrap_or_else(|| p.path()).display()
				);
				out.paths.push(p_path);
			}
			if let Some(ref mut a) = out.size_handles {
				a.push(tspawn(async move { p.metadata().ok().map(|m| m.len()) }));
			}
		}
		out
	})
}
