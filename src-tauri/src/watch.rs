use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub const EVENT: &str = "repo-changed";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Ignore,
    Worktree,
    Git,
}

fn canon(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn under(parent: &Path, child: &Path) -> bool {
    child.starts_with(parent)
}

pub fn classify(root: &Path, git_dir: &Path, path: &Path) -> Change {
    let root = canon(root);
    let git_dir = canon(git_dir);
    let path = canon(path);
    if under(&git_dir, &path) {
        let rel = match path.strip_prefix(&git_dir) {
            Ok(rel) => rel,
            Err(_) => return Change::Ignore,
        };
        let mut comps = rel.components();
        return match comps.next() {
            Some(Component::Normal(name)) => match name.to_string_lossy().as_ref() {
                "HEAD" | "packed-refs" | "MERGE_HEAD" | "REBASE_HEAD" | "CHERRY_PICK_HEAD" | "refs" => {
                    Change::Git
                }
                _ => Change::Ignore,
            },
            _ => Change::Ignore,
        };
    }
    if !under(&root, &path) {
        return Change::Ignore;
    }
    let rel = match path.strip_prefix(&root) {
        Ok(rel) => rel,
        Err(_) => return Change::Ignore,
    };
    let text = rel.to_string_lossy();
    if text.split(['/', '\\']).any(|part| part == "node_modules" || part == "target") {
        return Change::Ignore;
    }
    if text.ends_with(".lock") || text.ends_with('~') || text.ends_with(".swp") || text.ends_with(".tmp") {
        return Change::Ignore;
    }
    Change::Worktree
}

pub fn summarize(root: &Path, git_dir: &Path, paths: &[PathBuf]) -> Option<&'static str> {
    let mut git = false;
    let mut work = false;
    for path in paths {
        match classify(root, git_dir, path) {
            Change::Ignore => {}
            Change::Git => git = true,
            Change::Worktree => work = true,
        }
    }
    if git {
        Some("git")
    } else if work {
        Some("worktree")
    } else {
        None
    }
}

pub fn start(app: AppHandle, root: PathBuf, git_dir: PathBuf) -> Result<RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(|e| e.to_string())?;
    let root_c = canon(&root);
    let git_c = canon(&git_dir);
    watcher
        .watch(&root_c, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;
    if !git_c.starts_with(&root_c) {
        watcher
            .watch(&git_c, RecursiveMode::Recursive)
            .map_err(|e| e.to_string())?;
    }
    std::thread::Builder::new()
        .name("stashmango-watch".into())
        .spawn(move || {
            while let Ok(item) = rx.recv() {
                let Ok(event) = item else { continue };
                if matches!(event.kind, EventKind::Access(_)) {
                    continue;
                }
                let mut paths = event.paths;
                std::thread::sleep(Duration::from_millis(280));
                while let Ok(Ok(extra)) = rx.try_recv() {
                    paths.extend(extra.paths);
                }
                if let Some(kind) = summarize(&root_c, &git_c, &paths) {
                    let _ = app.emit(EVENT, kind);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::{classify, summarize, Change};
    use std::path::PathBuf;

    #[test]
    fn classifies_worktree_and_git_refs() {
        let root = PathBuf::from("C:/repo");
        let git = PathBuf::from("C:/repo/.git");
        assert_eq!(classify(&root, &git, &PathBuf::from("C:/repo/src/a.js")), Change::Worktree);
        assert_eq!(classify(&root, &git, &PathBuf::from("C:/repo/.git/HEAD")), Change::Git);
        assert_eq!(
            classify(&root, &git, &PathBuf::from("C:/repo/.git/refs/heads/main")),
            Change::Git
        );
        assert_eq!(
            classify(&root, &git, &PathBuf::from("C:/repo/.git/objects/aa/bb")),
            Change::Ignore
        );
        assert_eq!(
            classify(&root, &git, &PathBuf::from("C:/repo/node_modules/x")),
            Change::Ignore
        );
        assert_eq!(
            summarize(&root, &git, &[PathBuf::from("C:/repo/.git/HEAD"), PathBuf::from("C:/repo/a.txt")]),
            Some("git")
        );
    }
}
