//! Records what the converter was built from, for the version line every
//! build carries (see `src/version.rs`): the git commit (`-dirty` with
//! uncommitted changes to tracked files) and the version of the
//! CONscripter the repository pins (`engine/`).

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() {
    let commit = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain", "--untracked-files=no"]).is_some_and(|s| !s.is_empty());
    println!("cargo:rustc-env=CONV_COMMIT={commit}{}", if dirty { "-dirty" } else { "" });

    let header = "../../engine/src/cons/version.h";
    let engine = std::fs::read_to_string(header)
        .ok()
        .and_then(|h| h.lines().find_map(|l| l.strip_prefix("#define CONSCRIPTER_VERSION ").map(|v| v.trim().trim_matches('"').to_string())))
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=CONV_ENGINE_VERSION={engine}");

    // Again after a commit, a checkout, an edit or an engine bump.
    println!("cargo:rerun-if-changed={header}");
    println!("cargo:rerun-if-changed=..");
    let mut watched = vec!["HEAD".to_string(), "index".to_string(), "packed-refs".to_string()];
    watched.extend(git(&["symbolic-ref", "-q", "HEAD"]));
    for name in watched {
        if let Some(path) = git(&["rev-parse", "--git-path", &name]) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}
