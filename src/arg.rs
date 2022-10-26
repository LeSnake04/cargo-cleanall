use std::path::PathBuf;

use clap::{value_parser, Arg, ArgAction, ValueHint};

pub fn matches() -> clap::ArgMatches {
	clap::command!()
		.args([
			Arg::new("PATHS")
				.help("Path to search for projects to clean")
				.value_parser(value_parser!(PathBuf))
				.value_hint(ValueHint::DirPath)
				.required(true)
				.action(ArgAction::Append),
			Arg::new("hidden")
				.short('H')
				.long("hidden")
				.action(ArgAction::SetTrue)
				.help("Get size of and clean hidden folders"),
			Arg::new("dry")
				.long("dry")
				.short('d')
				.action(ArgAction::SetTrue)
				.help("Don't clean any files"),
			Arg::new("ignore")
				.long("ignore")
				.short('i')
				.value_parser(value_parser!(PathBuf))
				.help("Ignore projects (by folder name, not Cargo name)")
				.value_hint(ValueHint::DirPath)
				.action(ArgAction::Append),
			Arg::new("no-size")
				.long("no-size")
				.short('s')
				.action(ArgAction::SetTrue)
				.help("Don't calculate the size"),
		])
		.arg_required_else_help(true)
		.get_matches()
}
