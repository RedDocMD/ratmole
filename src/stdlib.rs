use std::{
    fs,
    path::{Path as StdPath, PathBuf},
};

use git2::{Commit, FetchOptions, Oid, Repository, Tag};

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
    let mut remote = repo.find_remote("origin")?;
    // Fetch including tags
    let mut fetch_options = FetchOptions::default();
    fetch_options.download_tags(git2::AutotagOption::All);
    remote.fetch(&[MAIN_BRANCH], Some(&mut fetch_options), None)?;

    // Merge remote branch
    // Refer: https://github.com/libgit2/libgit2sharp/blob/5055fbda8bb319eba100f5e418d5beed534b83bc/LibGit2Sharp/Repository.cs#L1232
    struct FetchHead {
        _ref_name: String,
        remote_url: String,
        target_id: Oid,
        _was_merged: bool,
    }

    let mut fetch_heads = Vec::new();
    repo.fetchhead_foreach(|name, url, target, merged| {
        fetch_heads.push(FetchHead {
            _ref_name: name.into(),
            remote_url: std::str::from_utf8(url).unwrap().into(),
            target_id: *target,
            _was_merged: merged,
        });
        true
    })?;

    let annotated_commits: Vec<_> = fetch_heads
        .iter()
        .map(|fetch_head| {
            repo.annotated_commit_from_fetchhead(
                MAIN_BRANCH,
                &fetch_head.remote_url,
                &fetch_head.target_id,
            )
            .unwrap()
        })
        .collect();

    let annotated_commits: Vec<_> = annotated_commits.iter().collect();
    repo.merge(&annotated_commits, None, None)?;

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
        repo_checkout_branch(&self.repo, MAIN_BRANCH).unwrap();
    }
}
