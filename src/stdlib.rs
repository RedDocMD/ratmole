use std::{
    fs,
    path::{Path as StdPath, PathBuf},
    process::Command,
};

use git2::{Repository, Tag};

use crate::error::{Error, Result};

fn init_base_dir<P: AsRef<StdPath>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        if !path.is_dir() {
            return Err(Error::PathAlreadyExists(format!(
                "{} exists and is not a directory",
                path.display(),
            )));
        }
    } else {
        fs::create_dir(path)?;
    }
    Ok(())
}

const REMOTE_URL: &'static str = "https://github.com/rust-lang/rust";
const BASE_DIRNAME: &'static str = ".ratmole";
const REPO_DIRNAME: &'static str = "rust-repo";
const GIT_COMMAND: &'static str = "/usr/bin/git";
const MAIN_BRANCH: &'static str = "master";
const REMOTE_REFSPEC: &'static str = "origin/master";

fn repo_clone() -> Result<()> {
    let mut base_dir =
        home::home_dir().ok_or_else(|| Error::HomeDirNotFound("home dir not found"))?;
    base_dir.push(BASE_DIRNAME);
    init_base_dir(&base_dir)?;

    // Clone directory
    let clone_output = Command::new(GIT_COMMAND)
        .current_dir(&base_dir)
        .args(&["clone", REMOTE_URL, REPO_DIRNAME])
        .output()
        .expect("Failed to run git clone");
    assert!(clone_output.status.success());

    let mut repo_dir = base_dir;
    repo_dir.push(REPO_DIRNAME);

    // Init submodules
    let backtrace_submod_output = Command::new(GIT_COMMAND)
        .current_dir(&repo_dir)
        .args(&["submodule", "update", "--init", "library/backtrace"])
        .output()
        .expect("Failed to git submodule init");
    assert!(backtrace_submod_output.status.success());
    let stdarch_submod_output = Command::new(GIT_COMMAND)
        .current_dir(&repo_dir)
        .args(&["submodule", "update", "--init", "library/stdarch"])
        .output()
        .expect("Failed to git submodule init");
    assert!(stdarch_submod_output.status.success());

    Ok(())
}

fn repo_update<'repo, P: AsRef<StdPath>>(repo_dir: P) -> Result<()> {
    // Fetch including tags
    let fetch_output = Command::new(GIT_COMMAND)
        .current_dir(repo_dir.as_ref())
        .args(&["fetch", "--tags"])
        .output()
        .expect("Failed to run git fetch");
    assert!(fetch_output.status.success());

    // Merge remote branch
    let merge_output = Command::new(GIT_COMMAND)
        .current_dir(repo_dir.as_ref())
        .args(&["merge", "--ff-only", REMOTE_REFSPEC])
        .output()
        .expect("Failed to git merge");
    assert!(merge_output.status.success());

    // Update submodules
    let backtrace_submod_output = Command::new(GIT_COMMAND)
        .current_dir(&repo_dir)
        .args(&["submodule", "update", "--remote", "library/backtrace"])
        .output()
        .expect("Failed to git submodule init");
    assert!(backtrace_submod_output.status.success());
    let stdarch_submod_output = Command::new(GIT_COMMAND)
        .current_dir(&repo_dir)
        .args(&["submodule", "update", "--remote", "library/stdarch"])
        .output()
        .expect("Failed to git submodule init");
    assert!(stdarch_submod_output.status.success());
    Ok(())
}

// Repo must be checked out to HEAD before calling this.
fn repo_get_latest_tag<'repo>(repo: &'repo Repository) -> Result<Tag<'repo>> {
    let mut tags = Vec::new();
    repo.tag_foreach(|id, _| {
        let tag = repo.find_tag(id).unwrap();
        tags.push(tag);
        true
    })?;

    let latest = tags.into_iter().max_by_key(|tag| {
        tag.target()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .time()
            .seconds()
    });

    Ok(latest.unwrap())
}

fn repo_checkout_tag<P: AsRef<StdPath>>(repo_dir: P, tag_name: &str) -> Result<()> {
    let checkout_output = Command::new(GIT_COMMAND)
        .current_dir(repo_dir.as_ref())
        .args(&["checkout", tag_name])
        .output()
        .expect("Failed to run git checkout");
    assert!(checkout_output.status.success());
    Ok(())
}

fn repo_checkout_branch<P: AsRef<StdPath>>(repo_dir: P, branch_name: &str) -> Result<()> {
    let checkout_output = Command::new(GIT_COMMAND)
        .current_dir(repo_dir.as_ref())
        .args(&["checkout", branch_name])
        .output()
        .expect("Failed to run git checkout");
    assert!(checkout_output.status.success());
    Ok(())
}

pub fn init_std_repo() -> Result<PathBuf> {
    let mut repo_dir =
        home::home_dir().ok_or_else(|| Error::HomeDirNotFound("home dir not found"))?;
    repo_dir.push(BASE_DIRNAME);
    repo_dir.push(REPO_DIRNAME);

    if !repo_dir.exists() {
        repo_clone()?;
    }

    repo_checkout_branch(&repo_dir, MAIN_BRANCH)?;
    repo_update(&repo_dir)?;

    let repo = Repository::open(&repo_dir)?;
    let latest_tag = repo_get_latest_tag(&repo)?;
    repo_checkout_tag(&repo_dir, latest_tag.name().unwrap())?;

    let mut lib_path = repo_dir;
    lib_path.push("library");
    lib_path.push("std");
    lib_path.push("src");
    lib_path.push("lib.rs");

    Ok(lib_path)
}

pub fn restore_std_repo() -> Result<()> {
    let mut repo_dir =
        home::home_dir().ok_or_else(|| Error::HomeDirNotFound("home dir not found"))?;
    repo_dir.push(BASE_DIRNAME);
    repo_dir.push(REPO_DIRNAME);

    repo_checkout_branch(&repo_dir, MAIN_BRANCH)
}
