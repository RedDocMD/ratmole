use std::{
    fs,
    path::{Path as StdPath, PathBuf},
};

use git2::{build::CheckoutBuilder, Commit, FetchOptions, ObjectType, Oid, Repository, Tag};
use log::debug;

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

const REMOTE_URL: &str = "https://github.com/rust-lang/rust";
const REMOTE_NAME: &str = "origin";
const BASE_DIRNAME: &str = ".ratmole";
const REPO_DIRNAME: &str = "rust-repo";
const MAIN_BRANCH: &str = "master";

fn repo_clone() -> Result<Repository> {
    let mut base_dir = home::home_dir().ok_or(Error::HomeDirNotFound("home dir not found"))?;
    base_dir.push(BASE_DIRNAME);
    init_base_dir(&base_dir)?;

    let mut repo_dir = base_dir;
    repo_dir.push(REPO_DIRNAME);

    let repo = Repository::clone(REMOTE_URL, repo_dir)?;
    repo_update_submodules(&repo)?;
    Ok(repo)
}

fn repo_update_submodules(repo: &Repository) -> Result<()> {
    let mut submodules = repo.submodules()?;
    let cloned_submodules = vec!["library/backtrace", "library/stdarch"];
    for submodule in &mut submodules {
        let name = submodule.path().display().to_string();
        if cloned_submodules.contains(&name.as_str()) {
            println!("Updating submodule {}", name);
            submodule.update(true, None)?;
        }
    }
    Ok(())
}

fn repo_checkout_tag(repo: &Repository, tag: &Tag) -> Result<()> {
    let tag_commit = tag.target().unwrap().peel_to_commit().unwrap();
    repo_checkout_commit(repo, &tag_commit)
}

fn repo_checkout_commit(repo: &Repository, commit: &Commit) -> Result<()> {
    let treeish = repo.revparse_single(&commit.id().to_string())?;
    repo.checkout_tree(&treeish, None)?;
    repo.set_head_detached(commit.id())?;
    repo_update_submodules(repo)
}

fn repo_checkout_branch(repo: &Repository, branch_name: &str) -> Result<()> {
    let branch = repo.revparse_single(branch_name)?;
    repo.checkout_tree(&branch, None)?;
    repo.set_head(&format!("refs/heads/{}", branch_name))?;
    repo_update_submodules(repo)
}

fn repo_update(repo: &Repository) -> Result<()> {
    let mut remote = repo.find_remote(REMOTE_NAME)?;
    // Fetch including tags
    let mut fetch_options = FetchOptions::default();
    fetch_options.download_tags(git2::AutotagOption::All);
    remote.fetch(&[MAIN_BRANCH], Some(&mut fetch_options), None)?;

    // Merge remote branch (fast-forward only)
    // Refer: https://github.com/libgit2/libgit2sharp/blob/5055fbda8bb319eba100f5e418d5beed534b83bc/LibGit2Sharp/Commands/Pull.cs#L18
    // Refer: https://github.com/libgit2/libgit2sharp/blob/5055fbda8bb319eba100f5e418d5beed534b83bc/LibGit2Sharp/Repository.cs#L1232
    #[derive(Debug)]
    struct MergedFetchHead {
        _ref_name: String,
        remote_url: String,
        target_id: Oid,
    }

    let mut merged_fetch_heads = Vec::new();
    repo.fetchhead_foreach(|name, url, target, merged| {
        if merged {
            merged_fetch_heads.push(MergedFetchHead {
                _ref_name: name.into(),
                remote_url: std::str::from_utf8(url).unwrap().into(),
                target_id: *target,
            });
        }
        true
    })?;
    assert!(
        merged_fetch_heads.len() == 1,
        "expected to have one merged fetch-head after fetch"
    );
    for f in &merged_fetch_heads {
        debug!("{:?}", f);
    }

    // Refer: https://github.com/libgit2/libgit2/blob/b7bad55e4bb0a285b073ba5e02b01d3f522fc95d/examples/merge.c#L111
    let fetch_head = &merged_fetch_heads[0];
    let mut head = repo.head()?;
    let target_obj = repo.find_object(fetch_head.target_id, Some(ObjectType::Commit))?;

    // Perform fast-forward
    repo.checkout_tree(&target_obj, Some(&mut CheckoutBuilder::default()))?;
    head.set_target(fetch_head.target_id, "")?;
    debug!("HEAD now points to {}", fetch_head.target_id);

    // Update submodules
    repo_update_submodules(repo)?;
    Ok(())
}

fn repo_get_latest_tag(repo: &Repository) -> Result<Tag> {
    let mut tags = Vec::new();
    repo.tag_foreach(|id, _| {
        let tag = repo.find_tag(id).unwrap();
        tags.push(tag);
        true
    })?;
    let latest = tags
        .into_iter()
        .max_by_key(|tag| tag.target().unwrap().peel_to_commit().unwrap().time());

    Ok(latest.unwrap())
}

pub struct StdRepo {
    crate_path: PathBuf,
    repo: Repository,
}

impl StdRepo {
    pub fn new() -> Result<Self> {
        let mut repo_dir = home::home_dir().ok_or(Error::HomeDirNotFound("home dir not found"))?;
        repo_dir.push(BASE_DIRNAME);
        repo_dir.push(REPO_DIRNAME);

        let repo = if !repo_dir.exists() {
            repo_clone()?
        } else {
            Repository::open(&repo_dir)?
        };

        repo_checkout_branch(&repo, MAIN_BRANCH)?;
        repo_update(&repo)?;

        {
            let latest_tag = repo_get_latest_tag(&repo)?;
            repo_checkout_tag(&repo, &latest_tag)?;
        }

        let mut lib_path = repo_dir.clone();
        lib_path.push("library");
        lib_path.push("std");
        lib_path.push("src");
        lib_path.push("lib.rs");

        let mut crate_path = repo_dir.clone();
        crate_path.push("library");
        crate_path.push("std");

        Ok(Self { crate_path, repo })
    }

    pub fn crate_path(&self) -> &PathBuf {
        &self.crate_path
    }
}

impl Drop for StdRepo {
    fn drop(&mut self) {
        debug!("Dropping StdRepo");
        repo_checkout_branch(&self.repo, MAIN_BRANCH).unwrap();
    }
}
