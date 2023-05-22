use std::{path, process};

fn print_help() {
    println!(
        r#"Download GitHub release assets

Arguments
	-r, --repository        repository to download from
	-t, --tag               tag of release, default is latest
	-a, --asset-pattern     pattern of the asset to download
	
Flags
	-p, --prerelease        prereleases will be allowed
	-q, --quiet             will not print anything to stdout and stderr
	-h, --help              print this help
	-V, --version           show version number
	-f, --print-filename    prints asset's filename to stdout
"#
    );
}

fn print_version() {
    println!(env!("CARGO_PKG_VERSION"));
}

pub struct ParsedArgs {
    pub bin_path: String,
    pub bin_name: String,

    pub repository: String,
    pub asset_pattern: String,

    // with default values
    pub tag: String,

    // flags
    pub allow_prerelease: bool,
    pub quiet: bool,
    pub help: bool,
    pub version: bool,
    pub print_filename: bool,
}

#[allow(dead_code)]
struct UnverfiedArgs {
    bin_path: String,
    bin_name: String,

    repository_option: Option<String>,
    asset_pattern_option: Option<String>,

    quiet: bool,
    help: bool,
    version: bool,
}

fn verify_args(unverified_args: UnverfiedArgs) -> (String, String) {
    if unverified_args.help {
        print_help();
        process::exit(0);
    }

    if unverified_args.version {
        print_version();
        process::exit(0);
    }

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

pub fn parse_args(args: Vec<String>) -> ParsedArgs {
    let bin_path = args.get(0).unwrap_or(&"grd".to_string()).to_owned();
    let bin_name = path::Path::new(&bin_path)
        .file_name()
        // is only None if bin_path is "..", which is impossible
        // for executables, so unwraping is ok
        .unwrap()
        .to_string_lossy()
        .to_string();

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
                repository_option = next_option.map(|hi| hi.to_owned());
                i += 1;
            }
            "--asset-pattern" | "-a" => {
                asset_pattern_option = next_option.map(|hi| hi.to_owned());
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

    let unverified_args = UnverfiedArgs {
        bin_path: bin_path.clone(),
        bin_name: bin_name.clone(),
        repository_option,
        asset_pattern_option,
        quiet,
        help,
        version,
    };
    let verified_args = verify_args(unverified_args);

    return ParsedArgs {
        bin_path,
        bin_name,
        repository: verified_args.0,
        asset_pattern: verified_args.1,
        tag,
        allow_prerelease,
        quiet,
        help,
        version,
        print_filename,
    };
}
