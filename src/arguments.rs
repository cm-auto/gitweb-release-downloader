use std::{fmt::Display, num::NonZeroUsize};

use clap::{Args, FromArgMatches, Parser, Subcommand, ValueEnum};
use regex::Regex;

#[derive(Parser)]
#[clap(args_conflicts_with_subcommands = true, version)]
struct ArgumentsPrivate {
    #[clap(
        short = 'q',
        long = "quiet",
        default_value_t = false,
        help = "Do not print anything"
    )]
    quiet: bool,
    #[clap(subcommand)]
    command_mode: Option<CommandMode>,
    // this is always None. It seems like flattening an Option struct,
    // which contains a flatten itself is not possible.
    // The reason this is still here is that the help message includes
    // the arguments of download, if no subcommand is specified.
    #[clap(flatten)]
    download: Option<DownloadArgs>,
}

impl ArgumentsPrivate {
    pub fn command_mode(self) -> CommandMode {
        match self.command_mode {
            Some(args) => args,
            None => CommandMode::Download(self.download.unwrap()),
        }
    }
}

pub struct Arguments {
    pub quiet: bool,
    pub command_mode: CommandMode,
}

impl From<ArgumentsPrivate> for Arguments {
    fn from(val: ArgumentsPrivate) -> Self {
        Arguments {
            quiet: val.quiet,
            command_mode: val.command_mode(),
        }
    }
}

// see ArgumentsPrivate to find out why the "download" subcommand is inserted,
// if no subcommand is specified
pub fn parse_arguments() -> Arguments {
    let mut args_raw = std::env::args().collect::<Vec<_>>();
    if args_raw.len() > 2 {
        let first_arg = &args_raw[1];
        match first_arg.as_str() {
            // if the first argument is already a command, do nothing
            "download" | "query" 
            // do not insert if the first argument is any of the following
            | "help" | "--help" | "-h" | "--version" | "-V" => {}
            _ => {
                args_raw.insert(1, "download".to_string());
            }
        }
    }

    ArgumentsPrivate::parse_from(args_raw).into()
}

#[derive(Subcommand)]
pub enum CommandMode {
    #[clap(about = "Mode to download an asset (default if no subcommand is specified)")]
    Download(DownloadArgs),
    #[clap(about = "Mode to query information about assets or releases of a repository")]
    Query(QueryArgs),
}

#[derive(Args)]
pub struct DownloadArgs {
    #[clap(flatten)]
    pub repository: Repository,
    #[clap(
        short = 'a',
        long = "asset-pattern",
        help = "Regex pattern of the asset to download\nIf pattern matches multiple assets, the first matching will be downloaded"
    )]
    pub asset_pattern: String,

    #[clap(
        short = 't',
        long = "tag",
        help = "Tag of the release (latest if omitted)"
    )]
    pub tag: Option<String>,

    #[clap(
        short = 'p',
        long = "prerelease",
        default_value_t = false,
        help = "Include prereleases"
    )]
    pub allow_prerelease: bool,
    #[clap(
        short = 'f',
        long = "print-filename",
        default_value_t = false,
        help = "Print downloaded filename to stdout\nThis will not print, if --quiet is specified"
    )]
    pub print_filename: bool,
}

#[derive(Args)]
pub struct QueryArgs {
    #[command(subcommand)]
    pub query_type: QueryType,
}

#[derive(Subcommand)]
pub enum QueryType {
    Releases(ReleasesQueryArgs),
    Assets(AssetsQueryArgs),
}

// TODO: is there a way to check if a regex is valid at compile time?
// currently this is ensured via unit test
fn get_github_optional_origin_and_repository_regex() -> Regex {
    // clippy actually checks for valid regex
    // however it is not enforced on compilation
    // try uncommenting this to see that clippy will complain about this
    // Regex::new(r"[^((https?://)?github.com/)?(?P<owner>[^/]+)/(?P<name>[^/]+)$").unwrap()

    // the origin is optional, since at this point the GitWebsite is known to be GitHub
    Regex::new(r"^((https?://)?github.com/)?(?P<owner>[^/]+)/(?P<name>[^/]+)$").unwrap()
}

#[cfg_attr(test, derive(Debug))]
enum ParseRepositoryError {
    InvalidRepository(String),
}

impl Display for ParseRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseRepositoryError::InvalidRepository(repository_string) => {
                write!(f, "invalid repository: {}", repository_string)
            }
        }
    }
}

// this function takes its arguments as owned values, because they will be moved into the Repository struct
fn parse_repository(
    repository_string: String,
    website_type: GitWebsite,
    sub_path: Option<String>,
) -> Result<Repository, ParseRepositoryError> {
    match website_type {
        GitWebsite::GitHub => {
            // since this function will only be called once
            // during the lifetime of the program, the regex pattern
            // will not be cached
            let github_pattern = get_github_optional_origin_and_repository_regex();
            let captures_option = github_pattern.captures(&repository_string);
            if let Some(captures) = captures_option {
                return Ok(Repository {
                    website: website_type,
                    owner: captures["owner"].to_string(),
                    name: captures["name"].to_string(),
                    sub_path,
                    passed_string: repository_string,
                });
            }
        }
    };
    Err(ParseRepositoryError::InvalidRepository(repository_string))
}

#[derive(Args)]
pub struct ReleasesQueryArgs {
    #[clap(flatten)]
    pub repository: Repository,
    #[clap(
        short = 'p',
        long = "prelease",
        default_value_t = false,
        help = "Include prereleases"
    )]
    pub allow_prerelease: bool,
    // by default one -> just latest
    #[clap(
        short = 'c',
        long = "count",
        default_value = "1",
        help = "The last n releases to show"
    )]
    pub count: NonZeroUsize,
}

#[derive(Args)]
pub struct AssetsQueryArgs {
    #[clap(flatten)]
    pub repository: Repository,
    #[clap(
        short = 't',
        long = "tag",
        help = "Tag of the release\nIf omitted latest tag will be used"
    )]
    pub tag: Option<String>,
    // ".*" means all assets
    #[clap(
        short = 'a',
        long = "asset-pattern",
        default_value = ".*",
        help = "Asset regex pattern to match against\nIf not supplied all assets will be shown"
    )]
    pub pattern: String,
}

#[derive(ValueEnum, Clone)]
#[cfg_attr(test, derive(PartialEq, Debug))]
#[clap(rename_all = "lower")]
pub enum GitWebsite {
    GitHub,
}

// RepositoryArguments takes the actual raw arguments passed to the
// program, while Repository is a "higher level" representation
// which has several values already parsed and/or extracted
#[derive(Parser)]
struct RepositoryArguments {
    // if website type and maybe sub path (depending on the website type) are specified
    // this does not need to be the full url
    #[clap(short = 'r', long = "repository", help = "Repository url")]
    pub repository: String,
    #[clap(
        short = 'w',
        long = "website-type",
        ignore_case = true,
        help = "If omitted, it will be guessed from repository url"
    )]
    pub website_type: Option<GitWebsite>,
    #[clap(
        short = 's',
        long = "sub-path",
        // only subpath not whole url
        help = "Sub path of the git website (https://example.com\x1b[1m/gitea\x1b[0m/user/repo -> /gitea)\nIgnored if website type is GitHub",
    )]
    pub sub_path: Option<String>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Repository {
    pub website: GitWebsite,
    pub owner: String,
    pub name: String,
    // for self hosted websites like Gitea
    // TODO: remove allow dead_code when sub_path is actually used
    #[allow(dead_code)]
    pub sub_path: Option<String>,
    pub passed_string: String,
}

impl FromArgMatches for Repository {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        RepositoryArguments::from_arg_matches(matches)?
            .try_into()
            .map_err(Into::into)
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        let repository_arguments = RepositoryArguments::from_arg_matches(matches)?;
        *self = repository_arguments
            .try_into()
            .map_err(Into::<clap::Error>::into)?;
        Ok(())
    }
}

impl Args for Repository {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        RepositoryArguments::augment_args(cmd)
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        RepositoryArguments::augment_args_for_update(cmd)
    }
}

// see get_github_optional_origin_and_repository_regex for notes to unwrapping Regex
fn get_guess_website_type_github_regex() -> Regex {
    Regex::new(r"^(https?://)?github.com/.*").unwrap()
}

fn guess_website_type(repository_string: &str) -> Option<GitWebsite> {
    let github_pattern = get_guess_website_type_github_regex();
    let captures_option = github_pattern.captures(repository_string);
    if captures_option.is_some() {
        return Some(GitWebsite::GitHub);
    }
    None
}

#[cfg_attr(test, derive(Debug))]
enum RepositoryArgumentsToRepositoryError {
    ParseRepository(ParseRepositoryError),
    GuessWebsiteFail,
}

impl From<ParseRepositoryError> for RepositoryArgumentsToRepositoryError {
    fn from(val: ParseRepositoryError) -> Self {
        Self::ParseRepository(val)
    }
}

impl From<RepositoryArgumentsToRepositoryError> for clap::Error {
    fn from(val: RepositoryArgumentsToRepositoryError) -> Self {
        use clap::error::ErrorKind::*;
        let (kind, message) = match val {
            RepositoryArgumentsToRepositoryError::ParseRepository(err) => {
                (ValueValidation, err.to_string())
            }
            RepositoryArgumentsToRepositoryError::GuessWebsiteFail => (
                MissingRequiredArgument,
                "failed to guess website type".to_string(),
            ),
        };
        clap::Error::raw(kind, message)
    }
}

impl TryFrom<RepositoryArguments> for Repository {
    type Error = RepositoryArgumentsToRepositoryError;

    fn try_from(val: RepositoryArguments) -> Result<Self, Self::Error> {
        let RepositoryArguments {
            repository,
            website_type,
            sub_path,
        } = val;

        // first we check if the website type has been provided as an argument
        // if not we try to guess it from the passed repository
        let website_type = match website_type {
            Some(website_type) => Some(website_type),
            None => guess_website_type(&repository),
        };

        // if it could not be guessed we return an error
        let website_type =
            website_type.ok_or(RepositoryArgumentsToRepositoryError::GuessWebsiteFail)?;

        let repository = parse_repository(repository, website_type, sub_path)?;
        Ok(repository)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guess_website_type() {
        assert!(matches!(
            guess_website_type("https://github.com/"),
            Some(GitWebsite::GitHub)
        ));
        assert!(guess_website_type("https://gitlab.com/").is_none());
    }

    #[test]
    fn test_parse_github_full_url_repository() {
        let repository = parse_repository(
            "https://github.com/cm-auto/gitweb-release-downloader".into(),
            GitWebsite::GitHub,
            None,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::GitHub,
            owner: "cm-auto".to_string(),
            name: "gitweb-release-downloader".to_string(),
            sub_path: None,
            passed_string: "https://github.com/cm-auto/gitweb-release-downloader".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_github_domain_and_repository() {
        let repository = parse_repository(
            "github.com/cm-auto/gitweb-release-downloader".into(),
            GitWebsite::GitHub,
            None,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::GitHub,
            owner: "cm-auto".to_string(),
            name: "gitweb-release-downloader".to_string(),
            sub_path: None,
            passed_string: "github.com/cm-auto/gitweb-release-downloader".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_github_only_repository() {
        let repository = parse_repository(
            "cm-auto/gitweb-release-downloader".into(),
            GitWebsite::GitHub,
            None,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::GitHub,
            owner: "cm-auto".to_string(),
            name: "gitweb-release-downloader".to_string(),
            sub_path: None,
            passed_string: "cm-auto/gitweb-release-downloader".to_string(),
        };
        assert_eq!(repository, expected);
    }

    // ===== regex checks =====
    // a test checks for panics
    // if the regex is valid, the test will succeed
    #[test]
    fn test_regex_compilations() {
        get_github_optional_origin_and_repository_regex();
        get_guess_website_type_github_regex();
    }
}
