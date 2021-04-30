//! ```cargo
//! [dependencies]
//! flate2 = "1.0.20"
//! tar = "0.4.33"
//! tempfile = "3.2.0"
//! ```
use flate2::GzBuilder;
use tempfile::NamedTempFile;

use std::env;
use std::fs;
use std::io;
use std::process::Command;

fn build(target: &str) {
    let mut cmd = Command::new("cross");
    cmd.args(&["build", "--release", "--target", target]);
    if env::var("CI").unwrap_or_default() == "true" {
        cmd.arg("--color=always");
    }
    assert!(cmd.status().unwrap().success());

    match fs::create_dir("release") {
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        result => result.unwrap(),
    }

    let file = NamedTempFile::new_in("release").unwrap();
    let writer = GzBuilder::new().write(&file, flate2::Compression::best());
    let mut archive = tar::Builder::new(writer);

    let path = format!("target/{}/release/zoxide", target);
    archive.append_path_with_name(&path, "zoxide").unwrap();

    const PATHS: &[&str] = &[
        "man/zoxide-add.1",
        "man/zoxide-import.1",
        "man/zoxide-init.1",
        "man/zoxide-query.1",
        "man/zoxide-remove.1",
        "man/zoxide.1",
        "CHANGELOG.md",
        "LICENSE",
        "README.md",
    ];
    for path in PATHS {
        archive.append_path(path).unwrap();
    }

    let version = env::var("CARGO_MAKE_PROJECT_VERSION").unwrap();
    let path = format!("release/zoxide-v{}-{}.tar.gz", version, target);

    drop(archive);
    file.persist(path).unwrap();
}

fn main() {
    const TARGETS: &[&str] = &[
        "aarch64-unknown-linux-gnu",
        "aarch64-unknown-linux-musl",
        "armv7-unknown-linux-gnueabihf",
        "armv7-unknown-linux-musleabihf",
        "x86_64-unknown-linux-gnu",
        "x86_64-unknown-linux-musl",
    ];

    let args = &mut env::args();
    match args.skip(1).next() {
        Some(target) => build(&target),
        None => {
            for target in TARGETS {
                build(target);
            }
        }
    }
    assert!(args.next().is_none());
}
