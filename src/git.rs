use git2::{Commit, Error, Oid, Reference, Repository};

pub fn clone_git_repository(repository_url: &str) -> Result<Repository, Error> {
    let git_path = tempfile::Builder::new()
        .prefix("git-clone-")
        .tempdir()
        .unwrap()
        .into_path();

    let repository = match Repository::clone(repository_url, git_path.as_path()) {
        Err(err) => {
            log::error!("Invalid repository");
            return Err(err);
        }
        Ok(repository) => repository,
    };
    log::info!(
        "Cloned repository to {}",
        repository.path().parent().unwrap().to_str().unwrap()
    );

    Ok(repository)
}

pub fn is_commit_head_or_head_ancestor(
    repository: &Repository,
    head: &Reference,
    commit_sha1: &str,
) -> bool {
    let sha1_oid = Oid::from_str(commit_sha1).unwrap();
    let head_oid = head.target().unwrap();
    if head_oid == sha1_oid {
        find_and_print_commit(repository, sha1_oid);
        return true;
    }

    match repository.graph_descendant_of(head_oid, sha1_oid) {
        Ok(is_descendant_of_head) => {
            if is_descendant_of_head {
                find_and_print_commit(repository, sha1_oid);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn find_and_print_commit(repository: &Repository, sha1: Oid) {
    let commit = repository.find_commit(sha1).unwrap();
    let commit_datetime = chrono::DateTime::from_timestamp(commit.time().seconds(), 0).unwrap();

    log::info!(
        "Sha1 commit was {} ({}): {}",
        commit.id(),
        commit_datetime.format("%d/%m/%Y %H:%M"),
        commit.message().unwrap().trim()
    );
}

pub fn find_commit_from_git_tag<'repo>(
    crate_version: &str,
    repository: &'repo Repository,
) -> Result<Commit<'repo>, ()> {
    log::warn!("No sha1 announced in crates.io, crate packaged with --allow-dirty");
    log::info!("Trying to find matching version tag on git repository");

    return match repository
        .tag_names(Some(crate_version))
        .or_else(|_| repository.tag_names(Some(format!("v{}", crate_version).as_str())))
    {
        Ok(tags) => match tags.get(0) {
            Some(tag) => {
                let tag_ref_name = format!("refs/tags/{}", tag);
                let tag_oid = repository.refname_to_id(tag_ref_name.as_str()).unwrap();
                match repository.find_commit(tag_oid) {
                    Ok(commit) => {
                        log::info!(
                            "Found matching version tag {} pointing to commit {}: {}",
                            tag,
                            commit.id(),
                            commit.summary().unwrap_or("No commit message")
                        );
                        Ok(commit)
                    }
                    Err(_) => {
                        log::error!("Couldnt find commit even though tag was found");
                        Err(())
                    }
                }
            }
            None => {
                log::error!("No matching version tag found");
                Err(())
            }
        },
        Err(_) => {
            log::error!("No matching version tag found");
            Err(())
        }
    };
}
