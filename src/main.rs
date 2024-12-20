mod arguments;
mod models;
use std::{
    fs::File,
    io::{stderr, Write},
    net::ToSocketAddrs,
    process::{self, exit},
};

use arguments::{GitWebsite, IpType};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use models::*;
use regex::Regex;
use ureq::{Agent, Resolver, Response};

impl Resolver for IpType {
    fn resolve(&self, netloc: &str) -> std::io::Result<Vec<std::net::SocketAddr>> {
        ToSocketAddrs::to_socket_addrs(netloc).map(|iter| {
            iter.filter(|address| match self {
                Self::Any => true,
                Self::IPV4 => address.is_ipv4(),
                Self::IPV6 => address.is_ipv6(),
            })
            .collect()
        })
    }
}

fn get_default_agent(repository: &arguments::Repository) -> Agent {
    ureq::AgentBuilder::new()
        .resolver(repository.ip_type)
        .build()
}

// GitHub requires the usage of a user agent
const USERAGENT: &str = "gitweb-release-downloader";

fn find_release<'a>(
    releases: &'a [Release],
    tag: Option<&str>,
    allow_prerelease: bool,
) -> Option<&'a Release> {
    for release in releases {
        if release.prerelease && !allow_prerelease {
            continue;
        }
        // if tag is latest take the first, which is
        // the latest
        match tag {
            None => return Some(release),
            Some(tag) => {
                if release.tag_name == tag {
                    return Some(release);
                }
            }
        }
    }
    None
}

fn find_asset<'a>(
    releases: &'a [Release],
    tag: Option<&str>,
    allow_prerelease: bool,
    asset_name_pattern: &Regex,
) -> Option<&'a Asset> {
    let release = find_release(releases, tag, allow_prerelease)?;
    release
        .assets
        .iter()
        .find(|&asset| asset_name_pattern.is_match(&asset.name))
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

#[inline(always)]
fn get_scheme_from_repository_string(url: &str) -> &str {
    if url.starts_with("http://") {
        "http"
    } else {
        "https"
    }
}

fn get_releases_api_url(repository: &arguments::Repository) -> String {
    let scheme = get_scheme_from_repository_string(&repository.passed_string);
    match repository.website {
        arguments::GitWebsite::GitHub => {
            format!(
                "{scheme}://api.github.com/repos/{owner}/{name}/releases",
                owner = repository.owner,
                name = repository.name,
            )
        }
        arguments::GitWebsite::Gitea => format!(
            "{scheme}://{origin}{sub_path}api/v1/repos/{owner}/{name}/releases",
            origin = repository.origin,
            sub_path = repository.sub_path,
            owner = repository.owner,
            name = repository.name
        ),
        arguments::GitWebsite::GitLab => format!(
            "{scheme}://{origin}{sub_path}api/v4/projects/{owner}%2F{name}/releases",
            origin = repository.origin,
            sub_path = repository.sub_path,
            owner = repository.owner,
            name = repository.name
        ),
    }
}

fn get_releases(agent: &Agent, repository: &arguments::Repository) -> Vec<Release> {
    let releases_address = get_releases_api_url(repository);

    let response =
        make_get_request(agent, &releases_address, &repository.headers).unwrap_or_else(|e| {
            eprintln!("HTTP request failed:\n{e}");
            process::exit(1);
        });

    let releases_json_string = response.into_string().unwrap_or_else(|e| {
        eprintln!("Could not get json from response:\n{e}");
        process::exit(1);
    });

    let releases = match repository.website {
        arguments::GitWebsite::GitHub | arguments::GitWebsite::Gitea => {
            serde_json::from_str::<Vec<Release>>(&releases_json_string)
        }
        arguments::GitWebsite::GitLab => {
            serde_json::from_str::<Vec<GitLabRelease>>(&releases_json_string)
                .map(|e| e.into_iter().map(Into::into).collect())
        }
    };
    releases.unwrap_or_else(|e| {
        eprintln!("Could not deserialize json:\n{e}");
        process::exit(1);
    })
}

fn get_compiled_asset_pattern_or_exit(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| {
        eprintln!("Could not compile RegEx:\n{e}");
        process::exit(1);
    })
}

fn get_asset_or_exit<'a>(
    releases: &'a [Release],
    parsed_args: &arguments::DownloadArgs,
    compiled_asset_pattern: &Regex,
) -> &'a Asset {
    let asset_option = find_asset(
        releases,
        parsed_args.tag.as_deref(),
        parsed_args.allow_prerelease,
        compiled_asset_pattern,
    );

    let Some(asset) = asset_option else {
        let tag_string = match &parsed_args.tag {
            Some(tag) => format!("tag \"{tag}\""),
            None => "latest tag".to_string(),
        };
        eprintln!(
            r#"Could not find Pattern "{asset_pattern}" in {tag_string} in releases of repository "{repository}""#,
            asset_pattern = parsed_args.asset_pattern,
            repository = parsed_args.repository.passed_string,
        );
        process::exit(1);
    };

    asset
}

fn make_get_request(
    agent: &Agent,
    url: &str,
    headers: &[String],
) -> Result<Response, Box<ureq::Error>> {
    let mut request = agent.get(url).set("user-agent", USERAGENT);
    for header in headers {
        // according to the first paragraph of the following mdn site, whitespace before the value
        // is ignored, so we don't need to remove anything
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers
        let (header_name, value) = header.split_once(":").unwrap_or_else(|| {
            eprintln!("Http header \"{header}\" has invalid format, must be: \"header-name: header-value\"");
            process::exit(1);
        });
        request = request.set(header_name, value);
    }

    request.call().map_err(Box::new)
}

fn get_content_length(response: &Response) -> Option<usize> {
    response
        .header("content-length")
        .map_or_else(|| None, |input| input.parse::<usize>().ok())
}

fn create_progress_bar(content_length: usize) -> ProgressBar {
    let pb = ProgressBar::new(content_length as u64);
    let pb_style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.green/red}] {bytes}/{total_bytes}",
    )
    // this hard coded template will always succeed compiling,
    // so it's okay to unwrap
    .unwrap()
    .progress_chars("=>-");
    pb.set_style(pb_style);
    pb
}

fn create_and_init_progress_bar(content_length_option: Option<usize>) -> Option<ProgressBar> {
    let content_length = content_length_option?;
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
                // If it fails we can't notify the user anyway
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

                if let Some(ref pb) = pb_option {
                    pb.set_position(bytes_downloaded as u64);
                }
            }
        }
    }
}

fn print_releases(releases_query_args: arguments::ReleasesQueryArgs) {
    let agent: Agent = get_default_agent(&releases_query_args.repository);

    let repository: arguments::Repository = releases_query_args.repository;
    let releases = get_releases(&agent, &repository);
    let releases_iter = releases
        .iter()
        .filter(|release| !release.prerelease || releases_query_args.allow_prerelease)
        .take(releases_query_args.count.into());
    for release in releases_iter {
        println!("{}", release.tag_name);
    }
}

fn print_assets(assets_query_args: arguments::AssetsQueryArgs) {
    let agent: Agent = get_default_agent(&assets_query_args.repository);

    let releases = get_releases(&agent, &assets_query_args.repository);
    // if no tag is specified, prereleases are not allowed
    // however if a tag is specified, the user explictly chose
    // a tag that might be a prerelease, so in this case it
    // will be allowed
    let allow_prerelease = assets_query_args.tag.is_some();
    let Some(release) = find_release(
        &releases,
        assets_query_args.tag.as_deref(),
        allow_prerelease,
    ) else {
        match &assets_query_args.tag {
            Some(tag) => eprintln!("Could not find release with tag \"{tag}\""),
            None => eprintln!("Could not find latest tag"),
        }
        process::exit(1);
    };
    let regex = get_compiled_asset_pattern_or_exit(&assets_query_args.pattern);
    let assets = find_assets_in_release(release, &regex);
    for asset in assets {
        println!("{}", asset.name);
    }
}

fn get_github_asset_api_url(owner: &str, repository: &str, asset_id: i64) -> String {
    format!("https://api.github.com/repos/{owner}/{repository}/releases/assets/{asset_id}")
}

fn download_assets(mut download_args: arguments::DownloadArgs) {
    let compiled_asset_pattern = get_compiled_asset_pattern_or_exit(&download_args.asset_pattern);

    let repository = &download_args.repository;
    let agent: Agent = get_default_agent(repository);
    let releases = get_releases(&agent, repository);
    let asset = get_asset_or_exit(&releases, &download_args, &compiled_asset_pattern);

    // drop immutable reference and get a mutable reference
    let repository = &mut download_args.repository;

    // printing to stderr, since posix (or unix?)
    // says progress is written to stderr
    // this makes sense especially if we pipe the name
    // into a script: the script gets the downloaded
    // file name and the user can still see the progress
    eprintln!(r#"Downloading "{}""#, &asset.name);

    let url_buffer: String;
    let url = if matches!(repository.website, GitWebsite::GitHub) {
        repository
            .headers
            .push("Accept: application/octet-stream".to_string());
        url_buffer = get_github_asset_api_url(&repository.owner, &repository.name, asset.id);
        url_buffer.as_str()
    } else {
        &asset.browser_download_url
    };

    let response = make_get_request(&agent, url, &repository.headers).unwrap_or_else(|e| {
        eprintln!("Error downloading file:\n{e}");
        process::exit(1);
    });

    let out_filename = &asset.name;

    let out_file = File::create(out_filename).unwrap_or_else(|e| {
        eprintln!("Error creating file:\n{e}");
        process::exit(1);
    });

    eprintln!("Writing to file \"{}\"", &out_filename);

    let content_length_option = get_content_length(&response);
    let pb_option = create_and_init_progress_bar(content_length_option);

    stream_response_into_file(response, out_file, &pb_option);

    if let Some(ref pb) = pb_option {
        pb.finish();
        eprintln!();
    }

    eprintln!(r#"Successfully wrote to file "{}""#, &out_filename);
    if download_args.print_filename {
        print!(r#"{}"#, &out_filename)
    }
}

fn main() {
    let args = arguments::Arguments::parse();

    match args.command_mode {
        arguments::CommandMode::Query(query_args) => match query_args.query_type {
            arguments::QueryType::Releases(releases_query_args) => {
                print_releases(releases_query_args);
                exit(0);
            }
            arguments::QueryType::Assets(assets_query_args) => {
                print_assets(assets_query_args);
                exit(0);
            }
        },
        arguments::CommandMode::Download(download_args) => {
            download_assets(download_args);
            exit(0);
        }
    };
}
