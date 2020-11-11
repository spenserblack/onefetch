use crate::onefetch::{commit_info::CommitInfo, error::*};
use git2::{Repository, RepositoryOpenFlags, Status, StatusOptions, StatusShow};
use regex::Regex;
use std::path::Path;

pub struct Repo {
    repo: Repository,
}

impl Repo {
    pub fn new(repo_path: &str) -> Result<Self> {
        let repo = Repository::discover(repo_path)?;
        Ok(Self { repo })
    }
    pub fn get_work_dir(&self) -> Result<String> {
        if let Some(workdir) = self.work_dir()?.to_str() {
            Ok(workdir.to_string())
        } else {
            Err("invalid workdir".into())
        }
    }

    pub fn get_version(&self) -> Result<String> {
        let mut number_of_release: usize = 0;
        let mut version_name = String::new();
        let mut most_recent: i64 = 0;

        self.repo.tag_foreach(|id, name| {
            if let Ok(name) = String::from_utf8(name[10..name.len()].into()) {
                let mut current_time: i64 = 0;
                if let Ok(tag) = self.repo.find_tag(id) {
                    if let Ok(c) = self.repo.find_commit(tag.target_id()) {
                        current_time = c.time().seconds();
                    }
                } else if let Ok(c) = self.repo.find_commit(id) {
                    current_time = c.time().seconds();
                }
                if current_time > most_recent {
                    most_recent = current_time;
                    version_name = name;
                }

                number_of_release += 1;
                return true;
            }
            false
        })?;
        let mut res = String::new();

        if !version_name.is_empty() {
            if number_of_release > 1 {
                res = format!("{} ({} releases)", version_name, number_of_release);
            } else if number_of_release == 1 {
                res = format!("{} ({} release)", version_name, number_of_release);
            } else {
                res = format!("{}", version_name);
            }
        }

        Ok(res)
    }

    pub fn get_pending_changes(&self) -> Result<String> {
        let statuses = self.repo.statuses(Some(
            StatusOptions::default()
                .show(StatusShow::Workdir)
                .update_index(true)
                .include_untracked(true)
                .renames_head_to_index(true)
                .recurse_untracked_dirs(true),
        ))?;

        let mut deleted: usize = 0;
        let mut added: usize = 0;
        let mut modified: usize = 0;

        for e in statuses.iter() {
            let s: Status = e.status();
            if s.is_index_new() || s.is_wt_new() {
                added += 1;
            } else if s.is_index_deleted() || s.is_wt_deleted() {
                deleted += 1;
            } else {
                modified += 1;
            }
        }

        let mut result = String::new();
        if modified > 0 {
            result = format!("{}+-", modified)
        }

        if added > 0 {
            result = format!("{} {}+", result, added);
        }

        if deleted > 0 {
            result = format!("{} {}-", result, deleted);
        }

        Ok(result.trim().into())
    }

    pub fn get_name_and_url(&self) -> Result<(String, String)> {
        let config = self.repo.config()?;
        let mut remote_origin_url: Option<String> = None;
        let mut remote_url_fallback = String::new();
        let mut repository_name = String::new();
        let remote_regex = Regex::new(r"remote\.[a-zA-Z0-9]+\.url").unwrap();

        for entry in &config.entries(None).unwrap() {
            let entry = entry?;
            let entry_name = entry.name().unwrap();
            if entry_name == "remote.origin.url" {
                remote_origin_url = Some(entry.value().unwrap().to_string());
            } else if remote_regex.is_match(entry_name) {
                remote_url_fallback = entry.value().unwrap().to_string()
            }
        }

        let remote_url = if let Some(url) = remote_origin_url { url } else { remote_url_fallback };

        let name_parts: Vec<&str> = remote_url.split('/').collect();

        if !name_parts.is_empty() {
            let mut i = 1;
            while repository_name.is_empty() && i <= name_parts.len() {
                repository_name = name_parts[name_parts.len() - i].to_string();
                i += 1;
            }
        }

        if repository_name.contains(".git") {
            let repo_name = repository_name.clone();
            let parts: Vec<&str> = repo_name.split(".git").collect();
            repository_name = parts[0].to_string();
        }

        Ok((repository_name, remote_url))
    }

    pub fn get_current_commit_info(&self) -> Result<CommitInfo> {
        let head = self.repo.head()?;
        let head_oid = head.target().ok_or("")?;
        let refs = self.repo.references()?;
        let refs_info = refs
            .filter_map(|reference| match reference {
                Ok(reference) => match (reference.target(), reference.shorthand()) {
                    (Some(oid), Some(shorthand)) if oid == head_oid => {
                        Some(if reference.is_tag() {
                            String::from("tags/") + shorthand
                        } else {
                            String::from(shorthand)
                        })
                    }
                    _ => None,
                },
                Err(_) => None,
            })
            .collect::<Vec<String>>();
        Ok(CommitInfo::new(head_oid, refs_info))
    }
    fn work_dir(&self) -> Result<&Path> {
        self.repo.workdir().ok_or_else(|| "unable to query workdir".into())
    }
}

pub fn is_valid(repo_path: &str) -> Result<bool> {
    let repo = Repository::open_ext(repo_path, RepositoryOpenFlags::empty(), Vec::<&Path>::new());

    Ok(repo.is_ok() && !repo?.is_bare())
}
