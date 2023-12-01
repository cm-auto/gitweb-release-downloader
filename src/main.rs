mod arguments;
mod models;
use std::{
    env,
    fs::File,
    io::{stderr, Write},
    process,
};

use arguments::{
    parse_args, print_help, print_version, AssetsQueryArgs, BasicArgs, CommandMode, DownloadArgs,
    GitWebsite, QueryType, ReleasesQueryArgs, Repository,
};
use indicatif::{ProgressBar, ProgressStyle};
use models::*;
use regex::Regex;
use ureq::{Agent, Response};

// GitHub requires the usage of a user agent
const USERAGENT: &'static str = "gitweb-release-downloader";

fn find_release<'a>(
    releases: &'a Vec<Release>,
    tag: &str,
    allow_prerelease: bool,
) -> Option<&'a Release> {
    for release in releases {
        if release.prerelease && !allow_prerelease {
            continue;
        }
        // if tag is latest take the first, which is
        // the latest
        if release.tag_name == tag || tag == "latest" {
            return Some(release);
        }
    }
    return None;
}

fn find_asset<'a>(
    releases: &'a Vec<Release>,
    tag: &str,
    allow_prerelease: bool,
    asset_name_pattern: &Regex,
) -> Option<&'a Asset> {
    let release = find_release(releases, tag, allow_prerelease)?;
    for asset in &release.assets {
        if asset_name_pattern.is_match(&asset.name) {
            return Some(asset);
        }
    }
    None
}

fn find_assets_in_release<'a>(release: &'a Release, asset_name_pattern: &Regex) -> Vec<&'a Asset> {
    let mut matching_assets = vec![];
    for asset in &release.assets {
        if asset_name_pattern.is_match(&asset.name) {
            matching_assets.push(asset);
        }
    }
    matching_assets
}

fn get_releases_api_url(repository: &Repository) -> String {
    match repository.website {
        GitWebsite::GitHub => {
            format!(
                "https://api.github.com/repos/{}/{}/releases",
                repository.owner, repository.name
            )
        }
    }
}

fn get_releases(agent: &Agent, repository: &Repository, quiet: bool) -> Vec<Release> {
    let releases_address = get_releases_api_url(repository);

    let response = make_get_request(agent, &releases_address).unwrap_or_else(|e| {
        if !quiet {
            eprintln!("HTTP request failed:\n{e}");
        }
        process::exit(1);
    });

    let releases_json_string = response.into_string().unwrap_or_else(|e| {
        if !quiet {
            eprintln!("Could not get json from response:\n{e}");
        }
        process::exit(1);
    });

    let releases =
        serde_json::from_str::<Vec<Release>>(&releases_json_string).unwrap_or_else(|e| {
            if !quiet {
                eprintln!("Could not deserialize json:\n{e}");
            }
            process::exit(1);
        });
    return releases;
}

// TODO create error enum with type of errors
// so we can exit the program with the respective error code

// TODO use more functions instead of putting everything into main

fn get_compiled_asset_pattern_or_exit(basic_args: &BasicArgs, pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| {
        if !basic_args.quiet {
            eprintln!("Could not compile RegEx:\n{e}");
        }

        process::exit(1);
    })
}

fn get_asset_or_exit<'a>(
    releases: &'a Vec<Release>,
    basic_args: &BasicArgs,
    parsed_args: &DownloadArgs,
    compiled_asset_pattern: &Regex,
) -> &'a Asset {
    let asset_option = find_asset(
        releases,
        &parsed_args.tag,
        parsed_args.allow_prerelease,
        compiled_asset_pattern,
    );

    let Some(asset) = asset_option else {
        if !basic_args.quiet {
            eprintln!(
                // TODO this error is also shown if the repository does not exist, which can be misleading
                r#"Could not find Pattern "{asset_pattern}" in Tag "{tag}" in releases of repository "{repository}""#,
                asset_pattern = parsed_args.asset_pattern,
                tag = parsed_args.tag,
                repository = parsed_args.repository.passed_string,
            );
        }
        process::exit(1);
    };

    asset
}

fn make_get_request(agent: &Agent, url: &str) -> Result<Response, ureq::Error> {
    let request = agent.get(url).set("user-agent", USERAGENT);

    request.call()
}

fn get_content_length(response: &Response) -> Option<usize> {
    response
        .header("content-length")
        .map_or_else(|| None, |input| input.parse::<usize>().ok())
}

fn create_progress_bar(content_length: usize) -> ProgressBar {
    let pb = ProgressBar::new(content_length as u64);
    let pb_style =
    // ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.green/red}] {bytes}/{total_bytes} ({eta})")
    ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.green/red}] {bytes}/{total_bytes}",
    )
    // TODO I suppose this hard coded template will always succeed compiling,
    // so it's okay to unwrap, however check that
    .unwrap()
    // this causes a compiler bug
    // .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
    .progress_chars("=>-");
    pb.set_style(pb_style);
    pb
}

fn create_and_init_progress_bar(content_length_option: Option<usize>) -> Option<ProgressBar> {
    if content_length_option.is_none() {
        return None;
    }
    let content_length = content_length_option.unwrap();
    let pb = create_progress_bar(content_length);
    pb.set_position(0);
    Some(pb)
}

fn stream_response_into_file(
    response: Response,
    mut out_file: File,
    pb_option: &Option<ProgressBar>,
) {
    let mut stream = response.into_reader();

    let mut bytes_downloaded = 0;
    let mut buffer = [0_u8; 8192];

    let mut stderr_locked = stderr().lock();

    loop {
        let chunk_result = stream.read(&mut buffer);
        match chunk_result {
            Err(error) => {
                // can we even properly handle the potential error
                // of writeln! ?
                // If it fails we can't notify the use anyway
                writeln!(stderr_locked, "Error reading stream:\n{error}").unwrap();
                process::exit(1);
            }
            Ok(read_size) => {
                // download has finished
                if read_size == 0 {
                    break;
                }
                let file_write_result = out_file.write(&buffer[0..read_size]);
                if let Err(error) = file_write_result {
                    writeln!(stderr_locked, "Could not write to file:\n{error}").unwrap();
                    process::exit(1);
                }

                bytes_downloaded += read_size;

                // TODO where is pb actually writing, too?
                if let Some(ref pb) = pb_option {
                    pb.set_position(bytes_downloaded as u64);
                }
            }
        }
    }
}

fn print_releases(basic_args: &BasicArgs, releases_query_args: &ReleasesQueryArgs) {
    let agent: Agent = ureq::AgentBuilder::new().build();

    if basic_args.quiet {
        return;
    }
    let releases = get_releases(&agent, &releases_query_args.repository, basic_args.quiet);
    let releases_iter = releases
        .iter()
        .filter(|release| !release.prerelease || releases_query_args.allow_prerelease)
        .take(releases_query_args.count.into());
    for release in releases_iter {
        println!("{}", release.tag_name);
    }
}

fn print_assets(basic_args: &BasicArgs, assets_query_args: &AssetsQueryArgs) {
    let agent: Agent = ureq::AgentBuilder::new().build();

    if basic_args.quiet {
        return;
    }
    let releases = get_releases(&agent, &assets_query_args.repository, basic_args.quiet);
    let Some(release) = find_release(&releases, &assets_query_args.tag, true) else {
        if !basic_args.quiet {
            eprintln!(
                r#"Could not find release with tag "{}""#,
                assets_query_args.tag
            )
        }
        process::exit(1);
    };
    let regex = get_compiled_asset_pattern_or_exit(basic_args, &assets_query_args.pattern);
    let assets = find_assets_in_release(release, &regex);
    for asset in assets {
        println!("{}", asset.name);
    }
}

fn handle_query_command(basic_args: &BasicArgs, query_type: QueryType) -> ! {
    match query_type {
        QueryType::ReleasesQuery(releases_args) => print_releases(basic_args, &releases_args),
        QueryType::AssetsQuery(assets_args) => print_assets(basic_args, &assets_args),
    }
    process::exit(0)
}

fn main() {
    let command_mode = parse_args(env::args().collect());

    let (basic_args, parsed_args) = match command_mode {
        CommandMode::Help(_) => {
            print_help();
            process::exit(0);
        }
        CommandMode::Version(_) => {
            print_version();
            process::exit(0);
        }
        CommandMode::Download(basic_args, download_args) => (basic_args, download_args),
        CommandMode::Query(basic_args, query_type) => handle_query_command(&basic_args, query_type),
    };

    // TODO does the usage of quiet make sense?
    // there are only prints to stderr (progress
    // and error messages), does it even make
    // sense to suppress stderr messages?
    let quiet = basic_args.quiet;

    // enable ansi on windows terminals
    // I think indicatif does this automatically,
    // but just to be sure:
    // TODO check if indicatif enables ansi on windows terminals

    let compiled_asset_pattern =
        get_compiled_asset_pattern_or_exit(&basic_args, &parsed_args.asset_pattern);

    let agent: Agent = ureq::AgentBuilder::new().build();

    let releases = get_releases(&agent, &parsed_args.repository, quiet);

    let asset = get_asset_or_exit(
        &releases,
        &basic_args,
        &parsed_args,
        &compiled_asset_pattern,
    );

    if !quiet {
        // printing to stderr, since posix (or unix?)
        // says progress is written to stderr
        // this makes sense especially if we pipe the name
        // into a script: the script gets the downloaded
        // file name and the user can still see the progress
        eprintln!(r#"Downloading "{}""#, &asset.name);
    }

    let response = make_get_request(&agent, &asset.browser_download_url).unwrap_or_else(|e| {
        if !quiet {
            eprintln!("Error downloading file:\n{e}");
        }
        process::exit(1);
    });

    let out_filename = &asset.name;

    let out_file = File::create(&out_filename).unwrap_or_else(|e| {
        if !quiet {
            eprintln!("Error creating file:\n{e}");
        }
        process::exit(1);
    });

    if !quiet {
        eprintln!("Writing to file \"{}\"", &out_filename);
    }

    let content_length_option = get_content_length(&response);
    let pb_option = if quiet {
        None
    } else {
        create_and_init_progress_bar(content_length_option)
    };

    stream_response_into_file(response, out_file, &pb_option);

    if let Some(ref pb) = pb_option {
        pb.finish();
        eprintln!();
    }

    if !quiet {
        eprintln!(r#"Successfully wrote to file "{}""#, &out_filename);
        if parsed_args.print_filename {
            print!(r#"{}"#, &out_filename)
        }
    }
}
