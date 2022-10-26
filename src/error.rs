use miette::Diagnostic;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, Diagnostic)]
pub enum Error {
	#[error("Argument not set: {0}")]
	ArgNotSet(&'static str),
	#[error("Command failed to execute")]
	ExecutionFailed(#[from] std::io::Error),
	#[error("Failed to get argument: {0}")]
	GetArgFailed(&'static str),
	#[error("Failed to start logger")]
	InitLoggerFailed(#[from] flexi_logger::FlexiLoggerError),
	#[error("Failed to join async")]
	JoinError(#[from] tokio::task::JoinError),
	#[error("Manifest could not be converted into &str")]
	ManifestToStrFailed,
	#[error("Failed to get File Name of parent")]
	NoFileName,
	#[error("Parent not found")]
	ParentNotFound,
	#[error("Failed to parse Args")]
	ParseArgsError(#[from] clap::parser::MatchesError),
	#[error("Size not Calculated")]
	SizeNotCalculated,
	#[cfg(debug_assertions)]
	#[error("Skipped")]
	Skipped,
}
