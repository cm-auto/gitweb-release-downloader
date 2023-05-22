use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs

#[test]
fn no_args_fails_code_1() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("grd")?;

    cmd.assert().failure().code(1);

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
