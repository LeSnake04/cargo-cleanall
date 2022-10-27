# Cargo Cleanall
A simple tool to clean all cargo projects in a directory.

## Help
```
Cargo plugin to clean all cargo projects in a directory

Usage: cargo cleanall [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Path to search for projects to clean

Options:
  -H, --hidden           Get size of and clean hidden folders
  -d, --dry              Don't clean any files
  -i, --ignore <ignore>  Ignore folders
  -s, --no-size          Don't calculate the size
  -h, --help             Print help information
  -V, --version          Print version information
```

## Output
```
$ Cargo cleanall .
INFO [cargo_cleanall] 1.2 GB => 117.8 MB (-1.1 GB: -90.303 %)
```

## Features
+ Asyncronus scan and cleanup for fast performance
+ Show size from before and after as well as an percentage (Can be turned off).
+ Option to include hidden files
+ Option to ignore folders

## Roadmap
+ Bugfixes
+ Code cleanup
+ Optimizations
Code reviews, suggestions and bug reports are welcome.

## Logging
This crate uses [flexi_logger](www.docs.rs/flexyi_logger).
To enable verbose output run with enviroment variable `RUST_LOG="debug"`
