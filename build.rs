use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_hash.trim());

    let output = Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=short"])
        .output()
        .unwrap();
    let git_date = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_COMMIT_DATE={}", git_date.trim());
}
