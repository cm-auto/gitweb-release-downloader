use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

// Technically these tests are not required anymore,
// since they are testing functionality of clap and
// the tests of a project should be limited to the
// the code of the project.
// However, they have been here from before clap has been used
// and might be useful as examples in the future.

// it seems like the default error code for clap is 2 if no args are provided
#[test]
fn no_args_fails_code_2() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.assert().failure().code(2);

    Ok(())
}

#[test]
fn non_existing_flag_fails_code_2() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;
    cmd.arg("--non-existing-flag");

    cmd.assert().failure().code(2);

    Ok(())
}

#[test]
fn argument_dash_v_prints_version_from_cargo_toml() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.arg("-V");
    cmd.assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));

    Ok(())
}

// #### start help tests ####
// these tests will not right out prevent breaking changes,
// however they can help in detecting specific types of breaking changes

#[test]
fn help_is_as_expected() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.arg("--help");
    cmd.assert().success().code(0).stdout(
        r#"Usage: grd <COMMAND>

Commands:
  download  Download an asset
  query     Query information about assets or releases of a repository
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
"#,
    );

    Ok(())
}

#[test]
fn download_help_is_as_expected() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.arg("download");
    cmd.arg("--help");
    cmd.assert().success().code(0).stdout(
        r#"Download an asset

Usage: grd download [OPTIONS] <REPOSITORY> <ASSET_PATTERN>

Arguments:
  <REPOSITORY>     Repository url (scheme defaults to "https" unless explicitly set to "http" with "http://")
  <ASSET_PATTERN>  Regex pattern of the asset to download
                   If pattern matches multiple assets, the first matching will be downloaded

Options:
  -w, --website-type <WEBSITE_TYPE>  If omitted, it will be guessed from repository url [possible values: github, gitea, gitlab]
  -i, --ip-type <IP_TYPE>            IP address type to use [default: any] [possible values: any, ipv4, ipv6]
      --header <HEADERS>             Http header to use, can be specified multiple times
  -t, --tag <TAG>                    Tag of the release (latest if omitted)
  -p, --prerelease                   Include prereleases
  -f, --print-filename               Print downloaded filename to stdout
  -h, --help                         Print help
"#,
    );

    Ok(())
}

#[test]
fn query_help_is_as_expected() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.arg("query");
    cmd.arg("--help");
    cmd.assert().success().code(0).stdout(
        r#"Query information about assets or releases of a repository

Usage: grd query <COMMAND>

Commands:
  releases  Query releases
  assets    Query assets
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
"#,
    );

    Ok(())
}

#[test]
fn query_releases_help_is_as_expected() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.arg("query");
    cmd.arg("releases");
    cmd.arg("--help");
    cmd.assert().success().code(0).stdout(
        r#"Query releases

Usage: grd query releases [OPTIONS] <REPOSITORY>

Arguments:
  <REPOSITORY>  Repository url (scheme defaults to "https" unless explicitly set to "http" with "http://")

Options:
  -w, --website-type <WEBSITE_TYPE>  If omitted, it will be guessed from repository url [possible values: github, gitea, gitlab]
  -i, --ip-type <IP_TYPE>            IP address type to use [default: any] [possible values: any, ipv4, ipv6]
      --header <HEADERS>             Http header to use, can be specified multiple times
  -p, --prerelease                   Include prereleases
  -c, --count <COUNT>                The last n releases to show [default: 1]
  -h, --help                         Print help
"#,
    );

    Ok(())
}
#[test]
fn query_assets_help_is_as_expected() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.arg("query");
    cmd.arg("assets");
    cmd.arg("--help");
    cmd.assert().success().code(0).stdout(
        r#"Query assets

Usage: grd query assets [OPTIONS] <REPOSITORY>

Arguments:
  <REPOSITORY>  Repository url (scheme defaults to "https" unless explicitly set to "http" with "http://")

Options:
  -w, --website-type <WEBSITE_TYPE>  If omitted, it will be guessed from repository url [possible values: github, gitea, gitlab]
  -i, --ip-type <IP_TYPE>            IP address type to use [default: any] [possible values: any, ipv4, ipv6]
      --header <HEADERS>             Http header to use, can be specified multiple times
  -t, --tag <TAG>                    Tag of the release
                                     If omitted latest (non prerelease) tag will be used
  -a, --asset-pattern <PATTERN>      Asset regex pattern to match against
                                     If not supplied all assets will be shown [default: .*]
  -h, --help                         Print help
"#,
    );

    Ok(())
}
// #### end help tests ####
