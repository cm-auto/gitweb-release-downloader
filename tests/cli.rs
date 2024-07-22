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
