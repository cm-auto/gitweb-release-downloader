use std::{num::NonZeroUsize, path, process};

pub fn print_help() {
    println!(
        r#"Download GitHub release assets

General Flags
    -q, --quiet             will not print anything to stdout and stderr
    -h, --help              print this help
    -V, --version           show version number

Download (Default)
Arguments
    -r, --repository        repository to download from
    -t, --tag               tag of release, default is latest
    -a, --asset-pattern     pattern of the asset to download
Flags
    -p, --prerelease        prereleases will be allowed
    -f, --print-filename    prints asset's filename to stdout

Query Releases (first two arguments: query releases)
Arguments
    -r, --repository        repository to query from
    --count                 amount of releases to display,
                            has to be at least 1 (1 is default)
Flags
    -p, --prerelease        prereleases will be allowed

Query Assets (first two arguments: query assets)
Arguments
    -r, --repository        repository to query from
    -t, --tag               tag of release, default is latest
    -a, --asset-pattern     pattern of the asset match against
                            default is ".*" (all assets)
"#
    );
}

pub fn print_version() {
    println!(env!("CARGO_PKG_VERSION"));
}

// for now just a String,
// later it probably becomes
// a struct, which contains website,
// author/organization name and
// repository name etc.
type Repository = String;

#[derive(Debug, PartialEq)]
pub struct BasicArgs {
    pub name: String,
    pub path: String,
    pub quiet: bool,
}

#[derive(Debug, PartialEq)]
pub enum CommandMode {
    Help(BasicArgs),
    Version(BasicArgs),
    Download(BasicArgs, DownloadArgs),
    Query(BasicArgs, QueryType),
}

#[derive(Debug, PartialEq)]
pub enum QueryType {
    ReleasesQuery(ReleasesQueryArgs),
    AssetsQuery(AssetsQueryArgs),
}

#[derive(Debug, PartialEq)]
pub struct ReleasesQueryArgs {
    pub repository: Repository,
    pub allow_prerelease: bool,
    // by default one -> just latest
    pub count: NonZeroUsize,
}

#[derive(Debug, PartialEq)]
pub struct AssetsQueryArgs {
    pub repository: Repository,
    // by default latest
    pub tag: String,
    // by default ".*" -> everything
    pub pattern: String,
}

#[derive(Debug, PartialEq)]
pub struct DownloadArgs {
    pub repository: Repository,
    pub asset_pattern: String,

    // with default values
    pub tag: String,

    // flags
    pub allow_prerelease: bool,
    pub print_filename: bool,
}

struct UnverfiedDownloadArgs {
    repository_option: Option<String>,
    asset_pattern_option: Option<String>,
    quiet: bool,
}

// TODO this should return an Error (Result) instead of exiting
fn verify_args(unverified_args: UnverfiedDownloadArgs) -> (String, String) {
    // TODO if the full link is supplied strip everything
    // that is not the repository
    if unverified_args.repository_option.is_none() {
        if !unverified_args.quiet {
            eprintln!(r#"You must supply a value for repository"#);
            print_help();
        }
        process::exit(1);
    }

    if unverified_args.asset_pattern_option.is_none() {
        if !unverified_args.quiet {
            eprintln!(r#"You must supply a value for asset pattern"#);
            print_help();
        }
        process::exit(1);
    }

    return (
        unverified_args.repository_option.unwrap(),
        unverified_args.asset_pattern_option.unwrap(),
    );
}

fn parse_query_releases_args(bin_path: String, bin_name: String, args: &[String]) -> CommandMode {
    // #### arguments ####
    let mut repository_option: Option<String> = None;

    // with default values
    let mut count: NonZeroUsize = NonZeroUsize::new(1).unwrap();

    // flags
    let mut allow_prerelease = false;
    // quiet is required for the basic_args
    // of course in query mode it doesn't make
    // that much sense
    let mut quiet = false;
    let mut help = false;
    let mut version = false;

    let mut i = 0;
    while i < args.len() {
        let current = args[i].clone();
        let next_option = args.get(i + 1);

        match current.as_str() {
            // first check for flags
            "--prerelease" | "-p" => allow_prerelease = true,
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,

            "--count" => {
                if let Some(next) = next_option {
                    count = next
                        .parse::<usize>()
                        .ok()
                        .and_then(NonZeroUsize::new)
                        .unwrap_or_else(|| {
                            eprintln!(r#"Argument "{}" has to be an int bigger than 0"#, next);
                            print_help();
                            process::exit(1);
                        });
                } else {
                    eprintln!(r#"You must supply a value for --count"#);
                    print_help();
                    process::exit(1);
                }
                i += 1;
            }
            // args that need a value
            "--repository" | "-r" => {
                repository_option = next_option.map(|next| next.to_owned());
                i += 1;
            }
            _ => {
                eprintln!(r#"Unrecognized flag: "{}" "#, current);
                print_help();
                process::exit(1);
            }
        }
        i += 1;
    }

    let basic_args = BasicArgs {
        name: bin_name,
        path: bin_path,
        quiet,
    };
    if help {
        return CommandMode::Help(basic_args);
    }
    if version {
        return CommandMode::Version(basic_args);
    }

    let Some(repository) = repository_option else{
        eprintln!(r#"You must supply a value for repository"#);
        print_help();
        process::exit(1);
    };

    CommandMode::Query(
        basic_args,
        QueryType::ReleasesQuery(ReleasesQueryArgs {
            repository,
            allow_prerelease,
            count,
        }),
    )
}

fn parse_query_assets_args(bin_path: String, bin_name: String, args: &[String]) -> CommandMode {
    // #### arguments ####
    let mut repository_option: Option<String> = None;

    // with default values
    let mut tag = "latest".to_string();
    // another way to have a default value is to let
    // this one be an option and then in the end of
    // the function, when we need it, we can unwrap_or
    // it with a default value
    let mut asset_pattern_option: Option<String> = None;

    // flags
    // quiet is required for the basic_args
    // of course in query mode it doesn't make
    // that much sense
    let mut quiet = false;
    let mut help = false;
    let mut version = false;

    let mut i = 0;
    while i < args.len() {
        let current = args[i].clone();
        let next_option = args.get(i + 1);

        match current.as_str() {
            // first check for flags
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,

            // args that need a value
            "--repository" | "-r" => {
                repository_option = next_option.map(|next| next.to_owned());
                i += 1;
            }

            "--tag" | "-t" => {
                if let Some(next) = next_option {
                    tag = next.to_owned();
                }
                i += 1;
            }
            "--asset-pattern" | "-a" => {
                asset_pattern_option = next_option.map(|next| next.to_owned());
                i += 1
            }

            _ => {
                eprintln!(r#"Unrecognized flag: "{}" "#, current);
                print_help();
                process::exit(1);
            }
        }
        i += 1;
    }

    let basic_args = BasicArgs {
        name: bin_name,
        path: bin_path,
        quiet,
    };
    if help {
        return CommandMode::Help(basic_args);
    }
    if version {
        return CommandMode::Version(basic_args);
    }

    let Some(repository) = repository_option else{
        eprintln!(r#"You must supply a value for repository"#);
        print_help();
        process::exit(1);
    };

    CommandMode::Query(
        basic_args,
        QueryType::AssetsQuery(AssetsQueryArgs {
            repository,
            tag: tag,
            pattern: asset_pattern_option.unwrap_or(".*".to_string()),
        }),
    )
}

// TODO return result
pub fn parse_args(args: Vec<String>) -> CommandMode {
    let bin_path = args.get(0).unwrap_or(&"grd".to_string()).to_owned();
    let bin_name = path::Path::new(&bin_path)
        .file_name()
        // is only None if bin_path is "..", which is impossible
        // for executables, so unwraping is ok
        .unwrap()
        .to_string_lossy()
        .to_string();

    let potential_command_option = args.get(1);
    if matches!(
        potential_command_option,
        Some(potential_command)
        if potential_command == "query"
    ) {
        match args.get(2) {
            Some(x) if x == "releases" => {
                return parse_query_releases_args(bin_path, bin_name, &args[3..]);
            }
            Some(x) if x == "assets" => {
                return parse_query_assets_args(bin_path, bin_name, &args[3..]);
            }
            Some(x) => {
                eprintln!(r#"Unknown query type "{}""#, x);
                print_help();
                process::exit(1);
            }
            None => {
                eprintln!(
                    r#"You must specify the kind of query you want to do, after argument "query""#
                );
                print_help();
                process::exit(1);
            }
        }
    }

    // #### arguments ####
    let mut repository_option: Option<String> = None;
    let mut asset_pattern_option: Option<String> = None;

    // with default values
    let mut tag = "latest".to_string();

    // flags
    let mut allow_prerelease = false;
    let mut quiet = false;
    let mut help = false;
    let mut version = false;
    let mut print_filename = false;

    let mut i = 1;
    while i < args.len() {
        let current = args[i].clone();
        let next_option = args.get(i + 1);

        match current.as_str() {
            // first check for flags
            "--prerelease" | "-p" => allow_prerelease = true,
            "--quiet" | "-q" => quiet = true,
            "--help" | "-h" => help = true,
            "--version" | "-V" => version = true,
            "--print-filename" | "-f" => print_filename = true,

            "--tag" | "-t" => {
                if let Some(next) = next_option {
                    tag = next.to_owned();
                }
                i += 1;
            }
            // args that need a value
            "--repository" | "-r" => {
                repository_option = next_option.map(|next| next.to_owned());
                i += 1;
            }
            "--asset-pattern" | "-a" => {
                asset_pattern_option = next_option.map(|next| next.to_owned());
                i += 1
            }

            _ => {
                eprintln!(r#"Unrecognized flag: "{}" "#, current);
                print_help();
                process::exit(1);
            }
        }
        i += 1;
    }

    let basic_args = BasicArgs {
        name: bin_name,
        path: bin_path,
        quiet,
    };
    if help {
        return CommandMode::Help(basic_args);
    }
    if version {
        return CommandMode::Version(basic_args);
    }

    let unverified_args = UnverfiedDownloadArgs {
        repository_option,
        asset_pattern_option,
        quiet,
    };
    let verified_args = verify_args(unverified_args);

    let download_args = DownloadArgs {
        repository: verified_args.0,
        asset_pattern: verified_args.1,
        tag,
        allow_prerelease,
        print_filename,
    };

    CommandMode::Download(basic_args, download_args)
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use crate::arguments::{
        parse_args, AssetsQueryArgs, BasicArgs, CommandMode, DownloadArgs, QueryType,
        ReleasesQueryArgs,
    };

    #[test]
    fn parse_help() {
        let args = vec!["grd".to_string(), "--help".to_string()];
        let mode = parse_args(args);
        let expected = CommandMode::Help(BasicArgs {
            name: "grd".to_string(),
            path: "grd".to_string(),
            quiet: false,
        });
        assert_eq!(mode, expected);
    }

    #[test]
    fn parse_version() {
        let args = vec!["grd".to_string(), "--version".to_string()];
        let mode = parse_args(args);
        let expected = CommandMode::Version(BasicArgs {
            name: "grd".to_string(),
            path: "grd".to_string(),
            quiet: false,
        });
        assert_eq!(mode, expected);
    }

    #[test]
    fn parse_query_default_download() {
        let args = vec![
            "grd".to_string(),
            "-r".to_string(),
            "cm-auto/gitweb-release-downloader".to_string(),
            "-a".to_string(),
            ".*".to_string(),
        ];
        let mode = parse_args(args);
        let expected = CommandMode::Download(
            BasicArgs {
                name: "grd".to_string(),
                path: "grd".to_string(),
                quiet: false,
            },
            DownloadArgs {
                repository: "cm-auto/gitweb-release-downloader".to_string(),
                asset_pattern: ".*".to_string(),
                tag: "latest".to_string(),
                allow_prerelease: false,
                print_filename: false,
            },
        );
        assert_eq!(mode, expected);
    }

    #[test]
    fn parse_query_releases() {
        let args = vec![
            "grd".to_string(),
            "query".to_string(),
            "releases".to_string(),
            "-r".to_string(),
            "cm-auto/gitweb-release-downloader".to_string(),
        ];
        let mode = parse_args(args);
        let expected = CommandMode::Query(
            BasicArgs {
                name: "grd".to_string(),
                path: "grd".to_string(),
                quiet: false,
            },
            QueryType::ReleasesQuery(ReleasesQueryArgs {
                repository: "cm-auto/gitweb-release-downloader".to_string(),
                count: NonZeroUsize::new(1).unwrap(),
                allow_prerelease: false,
            }),
        );
        assert_eq!(mode, expected);
    }

    #[test]
    fn parse_query_assets() {
        let args = vec![
            "grd".to_string(),
            "query".to_string(),
            "assets".to_string(),
            "-r".to_string(),
            "cm-auto/gitweb-release-downloader".to_string(),
        ];
        let mode = parse_args(args);
        let expected = CommandMode::Query(
            BasicArgs {
                name: "grd".to_string(),
                path: "grd".to_string(),
                quiet: false,
            },
            QueryType::AssetsQuery(AssetsQueryArgs {
                repository: "cm-auto/gitweb-release-downloader".to_string(),
                tag: "latest".to_string(),
                pattern: ".*".to_string(),
            }),
        );
        assert_eq!(mode, expected);
    }
}
