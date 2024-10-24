use std::{fmt::Display, num::NonZeroUsize};

use clap::{Args, FromArgMatches, Parser, Subcommand, ValueEnum};
use regex::Regex;

// TODO create error enum with type of errors and display them on --help

#[derive(Parser)]
#[clap(version)]
pub struct Arguments {
    #[clap(subcommand)]
    pub command_mode: CommandMode,
}

#[derive(Subcommand)]
pub enum CommandMode {
    #[clap(about = "Download an asset")]
    Download(DownloadArgs),
    #[clap(about = "Query information about assets or releases of a repository")]
    Query(QueryArgs),
}

#[derive(Args)]
pub struct DownloadArgs {
    #[clap(flatten)]
    pub repository: Repository,
    #[clap(
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
        help = "Print downloaded filename to stdout"
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
    #[clap(about = "Query releases")]
    Releases(ReleasesQueryArgs),
    #[clap(about = "Query assets")]
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
    Regex::new(r"^((https?://)?github\.com/)?(?P<owner>[^/]+)/(?P<name>[^/]+)$").unwrap()
}

fn get_gitea_origin_sub_path_and_repository_regex() -> Regex {
    // this includes the port
    Regex::new(r"^(https?://)?(?P<origin>([^/]+\.)+[^/]+)(?P<sub_path>/(([^/]+)/)*)(?P<owner>[^/]+)/(?P<name>[^/]+)$").unwrap()
}

fn get_gitlab_origin_sub_path_and_repository_regex() -> Regex {
    // currently exactly the same
    get_gitea_origin_sub_path_and_repository_regex()
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
                    origin: "github.com".to_string(),
                    sub_path: "/".to_string(),
                    passed_string: repository_string,
                });
            }
        }
        GitWebsite::Gitea => {
            let gitea_pattern = get_gitea_origin_sub_path_and_repository_regex();
            let captures_option = gitea_pattern.captures(&repository_string);
            if let Some(captures) = captures_option {
                return Ok(Repository {
                    website: website_type,
                    owner: captures["owner"].to_string(),
                    name: captures["name"].to_string(),
                    origin: captures["origin"].to_string(),
                    sub_path: captures["sub_path"].to_string(),
                    passed_string: repository_string,
                });
            }
        }
        GitWebsite::GitLab => {
            let gitlab_pattern = get_gitlab_origin_sub_path_and_repository_regex();
            let captures_option = gitlab_pattern.captures(&repository_string);
            if let Some(captures) = captures_option {
                return Ok(Repository {
                    website: website_type,
                    owner: captures["owner"].to_string(),
                    name: captures["name"].to_string(),
                    origin: captures["origin"].to_string(),
                    sub_path: captures["sub_path"].to_string(),
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
        long = "prerelease",
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
        help = "Tag of the release\nIf omitted latest (non prerelease) tag will be used"
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
    Gitea,
    GitLab,
}

// RepositoryArguments takes the actual raw arguments passed to the
// program, while Repository is a "higher level" representation
// which has several values already parsed and/or extracted
#[derive(Parser)]
struct RepositoryArguments {
    // if website type and maybe sub path (depending on the website type) are specified
    // this does not need to be the full url
    #[clap(help = "Repository url")]
    pub repository: String,
    #[clap(
        short = 'w',
        long = "website-type",
        ignore_case = true,
        help = "If omitted, it will be guessed from repository url"
    )]
    pub website_type: Option<GitWebsite>,
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Repository {
    pub website: GitWebsite,
    pub owner: String,
    pub name: String,
    // for self hosted websites like Gitea
    pub origin: String,
    pub sub_path: String,
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

fn get_guess_website_type_gitlab_com_regex() -> Regex {
    Regex::new(r"^(https?://)?gitlab.com/.*").unwrap()
}

fn guess_website_type(repository_string: &str) -> Option<GitWebsite> {
    if get_guess_website_type_github_regex()
        .captures(repository_string)
        .is_some()
    {
        return Some(GitWebsite::GitHub);
    }

    if get_guess_website_type_gitlab_com_regex()
        .captures(repository_string)
        .is_some()
    {
        return Some(GitWebsite::GitLab);
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

        let repository = parse_repository(repository, website_type)?;
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
        assert!(matches!(
            guess_website_type("https://gitlab.com/"),
            Some(GitWebsite::GitLab)
        ));
    }

    #[test]
    fn test_parse_github_full_url_repository() {
        let repository = parse_repository(
            "https://github.com/cm-auto/gitweb-release-downloader".into(),
            GitWebsite::GitHub,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::GitHub,
            owner: "cm-auto".to_string(),
            name: "gitweb-release-downloader".to_string(),
            origin: "github.com".to_string(),
            sub_path: "/".to_string(),
            passed_string: "https://github.com/cm-auto/gitweb-release-downloader".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_github_domain_and_repository() {
        let repository = parse_repository(
            "github.com/cm-auto/gitweb-release-downloader".into(),
            GitWebsite::GitHub,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::GitHub,
            owner: "cm-auto".to_string(),
            name: "gitweb-release-downloader".to_string(),
            origin: "github.com".to_string(),
            sub_path: "/".to_string(),
            passed_string: "github.com/cm-auto/gitweb-release-downloader".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_github_only_repository() {
        let repository = parse_repository(
            "cm-auto/gitweb-release-downloader".into(),
            GitWebsite::GitHub,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::GitHub,
            owner: "cm-auto".to_string(),
            name: "gitweb-release-downloader".to_string(),
            origin: "github.com".to_string(),
            sub_path: "/".to_string(),
            passed_string: "cm-auto/gitweb-release-downloader".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_codeberg_forgejo() {
        let repository = parse_repository(
            "https://codeberg.org/forgejo/forgejo".into(),
            GitWebsite::Gitea,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "forgejo".to_string(),
            name: "forgejo".to_string(),
            origin: "codeberg.org".to_string(),
            sub_path: "/".to_string(),
            passed_string: "https://codeberg.org/forgejo/forgejo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_codeberg_forgejo_without_protocol() {
        let repository =
            parse_repository("codeberg.org/forgejo/forgejo".into(), GitWebsite::Gitea).unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "forgejo".to_string(),
            name: "forgejo".to_string(),
            origin: "codeberg.org".to_string(),
            sub_path: "/".to_string(),
            passed_string: "codeberg.org/forgejo/forgejo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_sub_domain() {
        let repository = parse_repository(
            "https://gitea.example.com/owner/repo".into(),
            GitWebsite::Gitea,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "owner".to_string(),
            name: "repo".to_string(),
            origin: "gitea.example.com".to_string(),
            sub_path: "/".to_string(),
            passed_string: "https://gitea.example.com/owner/repo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_sub_domain_without_protocol() {
        let repository =
            parse_repository("gitea.example.com/owner/repo".into(), GitWebsite::Gitea).unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "owner".to_string(),
            name: "repo".to_string(),
            origin: "gitea.example.com".to_string(),
            sub_path: "/".to_string(),
            passed_string: "gitea.example.com/owner/repo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_sub_path() {
        let repository = parse_repository(
            "https://example.com/gitea/owner/repo".into(),
            GitWebsite::Gitea,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "owner".to_string(),
            name: "repo".to_string(),
            origin: "example.com".to_string(),
            sub_path: "/gitea/".to_string(),
            passed_string: "https://example.com/gitea/owner/repo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_sub_path_without_protocol() {
        let repository =
            parse_repository("example.com/gitea/owner/repo".into(), GitWebsite::Gitea).unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "owner".to_string(),
            name: "repo".to_string(),
            origin: "example.com".to_string(),
            sub_path: "/gitea/".to_string(),
            passed_string: "example.com/gitea/owner/repo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_with_port() {
        let repository = parse_repository(
            "https://example.com:1337/owner/repo".into(),
            GitWebsite::Gitea,
        )
        .unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "owner".to_string(),
            name: "repo".to_string(),
            origin: "example.com:1337".to_string(),
            sub_path: "/".to_string(),
            passed_string: "https://example.com:1337/owner/repo".to_string(),
        };
        assert_eq!(repository, expected);
    }

    #[test]
    fn test_parse_gitea_with_port_without_protocol() {
        let repository =
            parse_repository("example.com:1337/owner/repo".into(), GitWebsite::Gitea).unwrap();
        let expected = Repository {
            website: GitWebsite::Gitea,
            owner: "owner".to_string(),
            name: "repo".to_string(),
            origin: "example.com:1337".to_string(),
            sub_path: "/".to_string(),
            passed_string: "example.com:1337/owner/repo".to_string(),
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
        get_gitea_origin_sub_path_and_repository_regex();
        get_guess_website_type_gitlab_com_regex();
    }
}
