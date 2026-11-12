use crate::gitrun::{self, GitOut};
use crate::graph::{self, CommitNode};
use crate::patch::{self, FileDiff};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

const PAGE: usize = 300;
const MAX_BYTES: usize = 1_048_576;
const MAX_LINES: usize = 20_000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileState {
    pub path: String,
    pub staged: bool,
    pub unstaged: bool,
    pub untracked: bool,
    pub conflict: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoInfo {
    pub path: String,
    pub branch: String,
    pub files: Vec<FileState>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiffResult {
    pub path: String,
    pub binary: bool,
    pub too_big: bool,
    pub size: u64,
    pub text: String,
    pub hunks: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogPage {
    pub commits: Vec<CommitNode>,
    pub open_lanes: Vec<Option<String>>,
    pub done: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Worktree {
    pub path: String,
    pub branch: String,
    pub head: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayStep {
    pub sha: String,
    pub action: String,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConflictText {
    pub path: String,
    pub too_big: bool,
    pub ours: String,
    pub theirs: String,
    pub result: String,
}

pub fn open_repo(git: &Path, path: &Path) -> Result<RepoInfo, String> {
    let root = path
        .canonicalize()
        .map_err(|_| "Esa carpeta no existe.".to_string())?;
    let inside = gitrun::run(
        git,
        &root,
        &["rev-parse", "--is-inside-work-tree"],
        &[],
        None,
    )?;
    if !inside.ok() || inside.text().trim() != "true" {
        return Err("Esa carpeta no es un repositorio git.".into());
    }
    let top = gitrun::run_ok(git, &root, &["rev-parse", "--show-toplevel"], &[], None)?;
    let top = PathBuf::from(top.text().trim());
    info(git, &top)
}

pub fn info(git: &Path, root: &Path) -> Result<RepoInfo, String> {
    let branch = gitrun::run(git, root, &["branch", "--show-current"], &[], None)?;
    let branch = if branch.ok() {
        branch.text().trim().to_string()
    } else {
        String::new()
    };
    Ok(RepoInfo {
        path: root.to_string_lossy().to_string(),
        branch,
        files: status(git, root)?,
    })
}

pub fn status(git: &Path, root: &Path) -> Result<Vec<FileState>, String> {
    let out = gitrun::run_ok(git, root, &["status", "--porcelain=v2", "-z"], &[], None)?;
    Ok(parse_status(&out.stdout))
}

pub fn parse_status(bytes: &[u8]) -> Vec<FileState> {
    let mut files = Vec::new();
    for record in bytes.split(|b| *b == 0) {
        if record.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(record);
        let kind = text.as_bytes().first().copied().unwrap_or(0) as char;
        match kind {
            '?' => {
                let path = text.trim_start_matches("? ").trim().to_string();
                if !path.is_empty() {
                    files.push(file_state(path, false, false, true, false));
                }
            }
            'u' => {
                if let Some(path) = field_path(&text, 10) {
                    files.push(file_state(path, false, false, false, true));
                }
            }
            '1' => {
                if let Some((xy, path)) = xy_and_path(&text, 8) {
                    push_xy(&mut files, path, &xy);
                }
            }
            '2' => {
                if let Some((xy, path)) = xy_and_path(&text, 9) {
                    push_xy(&mut files, path, &xy);
                }
            }
            _ => {}
        }
    }
    files
}

fn file_state(path: String, staged: bool, unstaged: bool, untracked: bool, conflict: bool) -> FileState {
    FileState {
        path,
        staged,
        unstaged,
        untracked,
        conflict,
    }
}

fn xy_and_path(text: &str, header_fields: usize) -> Option<(String, String)> {
    let mut parts = text.splitn(header_fields + 1, ' ');
    let mut header = Vec::new();
    for _ in 0..header_fields {
        header.push(parts.next()?);
    }
    let path = parts.next()?.to_string();
    Some((header.get(1)?.to_string(), path))
}

fn field_path(text: &str, header_fields: usize) -> Option<String> {
    text.splitn(header_fields + 1, ' ').nth(header_fields).map(|s| s.to_string())
}

fn push_xy(files: &mut Vec<FileState>, path: String, xy: &str) {
    let x = xy.chars().next().unwrap_or('.');
    let y = xy.chars().nth(1).unwrap_or('.');
    let staged = x != '.' && x != ' ';
    let unstaged = y != '.' && y != ' ';
    if let Some(existing) = files.iter_mut().find(|f| f.path == path) {
        existing.staged |= staged;
        existing.unstaged |= unstaged;
        return;
    }
    files.push(file_state(path, staged, unstaged, false, false));
}

pub fn log_page(
    git: &Path,
    root: &Path,
    skip: usize,
    mut open_lanes: Vec<Option<String>>,
) -> Result<LogPage, String> {
    let skip_s = skip.to_string();
    let take = (PAGE + 1).to_string();
    let out = gitrun::run_ok(
        git,
        root,
        &[
            "log",
            "--date-order",
            "--all",
            "-n",
            &take,
            "--skip",
            &skip_s,
            "--format=%H%x1f%P%x1f%D%x1f%an%x1f%ae%x1f%at%x1f%s%x1e",
        ],
        &[],
        None,
    )?;
    let text = out.text();
    let mut commits = Vec::new();
    for record in text.split('\u{1e}') {
        let record = record.trim_matches(['\n', '\r']);
        if record.is_empty() {
            continue;
        }
        let mut fields = record.split('\u{1f}');
        let Some(id) = fields.next() else { continue };
        if id.len() < 7 {
            continue;
        }
        let parents = fields.next().unwrap_or("").split_whitespace().map(|s| s.to_string()).filter(|s| !s.is_empty()).collect();
        let refs = fields.next().unwrap_or("").to_string();
        let author = fields.next().unwrap_or("").to_string();
        let email = fields.next().unwrap_or("").to_string();
        let time = fields.next().unwrap_or("0").parse().unwrap_or(0);
        let summary = fields.next().unwrap_or("").trim().to_string();
        commits.push(CommitNode {
            id: id.to_string(),
            parents,
            summary,
            author,
            email,
            time,
            refs,
            lane: 0,
            parent_lanes: Vec::new(),
        });
    }
    let done = commits.len() <= PAGE;
    commits.truncate(PAGE);
    graph::place(&mut commits, &mut open_lanes);
    Ok(LogPage {
        commits,
        open_lanes,
        done,
    })
}

pub fn diff_file(git: &Path, root: &Path, path: &str, cached: bool) -> Result<DiffResult, String> {
    let rel = relative_in_repo(path)?;
    let size = worktree_size(root, &rel);
    if size > MAX_BYTES as u64 {
        return Ok(too_big(rel, size));
    }
    let mut args = vec!["diff", "-U0", "--no-ext-diff", "--"];
    if cached {
        args = vec!["diff", "--cached", "-U0", "--no-ext-diff", "--"];
    }
    args.push(&rel);
    let out = gitrun::run_ok(git, root, &args, &[], None)?;
    let text = out.text();
    if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
        return Ok(too_big(rel, size));
    }
    let parsed = patch::parse(&text);
    Ok(DiffResult {
        path: rel,
        binary: parsed.binary,
        too_big: false,
        size,
        text: if parsed.binary { String::new() } else { text },
        hunks: parsed.hunks.len(),
    })
}

fn too_big(path: String, size: u64) -> DiffResult {
    DiffResult {
        path,
        binary: false,
        too_big: true,
        size,
        text: String::new(),
        hunks: 0,
    }
}

pub fn stage_file(git: &Path, root: &Path, path: &str, stage: bool) -> Result<(), String> {
    let rel = relative_in_repo(path)?;
    if stage {
        gitrun::run_ok(git, root, &["add", "--", &rel], &[], None)?;
    } else {
        gitrun::run_ok(git, root, &["restore", "--staged", "--", &rel], &[], None)?;
    }
    Ok(())
}

pub fn apply_selection(
    git: &Path,
    root: &Path,
    path: &str,
    cached: bool,
    hunks: &[usize],
    lines: &[(usize, usize)],
) -> Result<(), String> {
    let rel = relative_in_repo(path)?;
    let mut args = vec!["diff", "-U0", "--no-ext-diff", "--", rel.as_str()];
    if cached {
        args = vec!["diff", "--cached", "-U0", "--no-ext-diff", "--", rel.as_str()];
    }
    let out = gitrun::run_ok(git, root, &args, &[], None)?;
    let parsed = patch::parse(&out.text());
    if parsed.binary {
        return Err("Ese archivo es binario. Se puede añadir entero, no por líneas.".into());
    }
    let mut selected = hunks.to_vec();
    if !lines.is_empty() {
        selected.clear();
        let mut extra = Vec::new();
        for (hunk_index, line_index) in lines {
            let Some(hunk) = parsed.hunks.get(*hunk_index) else {
                continue;
            };
            let split = patch::split_lines(hunk);
            if let Some(piece) = split.get(*line_index) {
                extra.push(piece.clone());
            }
        }
        let rebuilt = FileDiff {
            preamble: parsed.preamble.clone(),
            hunks: extra,
            binary: false,
        };
        if rebuilt.hunks.is_empty() {
            return Err("No hay líneas que aplicar.".into());
        }
        let patch = patch::render(&rebuilt, &(0..rebuilt.hunks.len()).collect::<Vec<_>>());
        return apply_patch(git, root, &patch, cached);
    }
    if selected.is_empty() {
        return Err("No hay hunks que aplicar.".into());
    }
    let patch_text = patch::render(&parsed, &selected);
    apply_patch(git, root, &patch_text, cached)
}

fn apply_patch(git: &Path, root: &Path, patch_text: &str, reverse: bool) -> Result<(), String> {
    let mut args = vec!["apply", "--cached", "--unidiff-zero", "--whitespace=nowarn"];
    if reverse {
        args.push("-R");
    }
    args.push("-");
    gitrun::run_ok(git, root, &args, &[], Some(patch_text.as_bytes()))?;
    Ok(())
}

pub fn commit(git: &Path, root: &Path, message: &str, name: Option<&str>, email: Option<&str>) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("El commit necesita un mensaje.".into());
    }
    if message.contains('\0') {
        return Err("El mensaje no es válido.".into());
    }
    let mut env: Vec<(&str, &str)> = Vec::new();
    if let (Some(name), Some(email)) = (name, email) {
        crate::store::validate_identity(name, email)?;
        env.push(("GIT_AUTHOR_NAME", name));
        env.push(("GIT_AUTHOR_EMAIL", email));
        env.push(("GIT_COMMITTER_NAME", name));
        env.push(("GIT_COMMITTER_EMAIL", email));
    }
    gitrun::run_ok(git, root, &["commit", "-F", "-"], &env, Some(message.as_bytes()))?;
    Ok(())
}

pub fn local_config(git: &Path, root: &Path, key: &str) -> Result<String, String> {
    let out = gitrun::run(git, root, &["config", "--local", "--get", key], &[], None)?;
    if out.ok() {
        Ok(out.text().trim().to_string())
    } else {
        Ok(String::new())
    }
}

pub fn branches(git: &Path, root: &Path) -> Result<Vec<String>, String> {
    let out = gitrun::run_ok(git, root, &["for-each-ref", "--format=%(refname:short)", "refs/heads"], &[], None)?;
    Ok(out.text().lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
}

pub fn create_branch(git: &Path, root: &Path, name: &str) -> Result<(), String> {
    check_branch(git, root, name)?;
    gitrun::run_ok(git, root, &["switch", "-c", name], &[], None)?;
    Ok(())
}

pub fn switch_branch(git: &Path, root: &Path, name: &str) -> Result<(), String> {
    check_branch(git, root, name)?;
    gitrun::run_ok(git, root, &["switch", "--", name], &[], None)?;
    Ok(())
}

pub fn check_branch(git: &Path, root: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['\n', '\r', ' ']) {
        return Err("El nombre de la rama no es válido.".into());
    }
    let out = gitrun::run(git, root, &["check-ref-format", "--branch", name], &[], None)?;
    if out.ok() {
        Ok(())
    } else {
        Err("El nombre de la rama no es válido.".into())
    }
}

pub fn worktrees(git: &Path, root: &Path) -> Result<Vec<Worktree>, String> {
    let out = gitrun::run_ok(git, root, &["worktree", "list", "--porcelain"], &[], None)?;
    let mut list = Vec::new();
    let mut current = Worktree {
        path: String::new(),
        branch: String::new(),
        head: String::new(),
    };
    for line in out.text().lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            if !current.path.is_empty() {
                list.push(current);
            }
            current = Worktree {
                path: path.to_string(),
                branch: String::new(),
                head: String::new(),
            };
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current.head = head.to_string();
        } else if let Some(branch) = line.strip_prefix("branch ") {
            current.branch = branch.trim_start_matches("refs/heads/").to_string();
        }
    }
    if !current.path.is_empty() {
        list.push(current);
    }
    Ok(list)
}

pub fn add_worktree(git: &Path, root: &Path, path: &Path, branch: &str) -> Result<(), String> {
    check_branch(git, root, branch)?;
    let path_text = path.to_string_lossy().to_string();
    gitrun::run_ok(git, root, &["worktree", "add", &path_text, branch], &[], None)?;
    Ok(())
}

pub fn remove_worktree(git: &Path, root: &Path, path: &Path, confirm: bool) -> Result<(), String> {
    if !confirm {
        return Err("Quitar un worktree pide confirmación.".into());
    }
    let path_text = path.to_string_lossy().to_string();
    gitrun::run_ok(git, root, &["worktree", "remove", &path_text], &[], None)?;
    Ok(())
}

pub fn stash_list(git: &Path, root: &Path) -> Result<Vec<String>, String> {
    let out = gitrun::run_ok(git, root, &["stash", "list"], &[], None)?;
    Ok(out.text().lines().map(|l| l.to_string()).filter(|l| !l.is_empty()).collect())
}

pub fn stash_push(git: &Path, root: &Path, message: &str) -> Result<(), String> {
    if message.contains(['\n', '\r', '\0']) {
        return Err("El mensaje del stash no es válido.".into());
    }
    gitrun::run_ok(git, root, &["stash", "push", "-m", message], &[], None)?;
    Ok(())
}

pub fn stash_pop(git: &Path, root: &Path) -> Result<(), String> {
    gitrun::run_ok(git, root, &["stash", "pop"], &[], None)?;
    Ok(())
}

pub fn stash_drop(git: &Path, root: &Path, confirm: bool) -> Result<(), String> {
    if !confirm {
        return Err("Soltar un stash pide confirmación.".into());
    }
    gitrun::run_ok(git, root, &["stash", "drop"], &[], None)?;
    Ok(())
}

pub fn merge(git: &Path, root: &Path, branch: &str) -> Result<GitOut, String> {
    check_branch(git, root, branch)?;
    gitrun::run(git, root, &["merge", "--no-edit", branch], &[], None)
}

pub fn rebase(git: &Path, root: &Path, upstream: &str) -> Result<(), String> {
    check_branch(git, root, upstream)?;
    gitrun::run_ok(git, root, &["rebase", upstream], &[], None)?;
    Ok(())
}

pub fn cherry_pick(git: &Path, root: &Path, sha: &str) -> Result<(), String> {
    check_sha(sha)?;
    gitrun::run_ok(git, root, &["cherry-pick", sha], &[], None)?;
    Ok(())
}

pub fn replay(git: &Path, root: &Path, base: &str, steps: &[ReplayStep]) -> Result<(), String> {
    check_sha(base)?;
    let head = gitrun::run_ok(git, root, &["rev-parse", "HEAD"], &[], None)?;
    let saved = head.text().trim().to_string();
    if let Err(err) = gitrun::run_ok(git, root, &["reset", "--hard", base], &[], None) {
        return Err(err);
    }
    for step in steps {
        if step.action == "drop" {
            continue;
        }
        if step.action != "pick" && step.action != "reword" {
            let _ = gitrun::run(git, root, &["reset", "--hard", &saved], &[], None);
            return Err("La acción de rebase no es válida.".into());
        }
        check_sha(&step.sha)?;
        if let Err(err) = gitrun::run_ok(git, root, &["cherry-pick", &step.sha], &[], None) {
            return Err(err);
        }
        if step.action == "reword" {
            let message = step.message.clone().unwrap_or_default();
            if message.trim().is_empty() {
                return Err("Reescribir un commit pide un mensaje.".into());
            }
            gitrun::run_ok(git, root, &["commit", "--amend", "-F", "-"], &[], Some(message.as_bytes()))?;
        }
    }
    Ok(())
}

pub fn reset_hard(git: &Path, root: &Path, target: &str, confirm: bool) -> Result<String, String> {
    if !confirm {
        return Err("Reset hard pide confirmación.".into());
    }
    check_sha(target)?;
    let head = gitrun::run_ok(git, root, &["rev-parse", "HEAD"], &[], None)?;
    let previous = head.text().trim().to_string();
    gitrun::run_ok(git, root, &["reset", "--hard", target], &[], None)?;
    Ok(previous)
}

pub fn head_sha(git: &Path, root: &Path) -> Result<String, String> {
    let out = gitrun::run_ok(git, root, &["rev-parse", "HEAD"], &[], None)?;
    Ok(out.text().trim().to_string())
}

pub fn conflict_text(git: &Path, root: &Path, path: &str) -> Result<ConflictText, String> {
    let rel = relative_in_repo(path)?;
    let size = worktree_size(root, &rel);
    if size > MAX_BYTES as u64 {
        return Ok(ConflictText {
            path: rel,
            too_big: true,
            ours: String::new(),
            theirs: String::new(),
            result: String::new(),
        });
    }
    let ours = show_stage(git, root, &rel, 2).unwrap_or_default();
    let theirs = show_stage(git, root, &rel, 3).unwrap_or_default();
    let result = fs_read(root, &rel).unwrap_or_default();
    if ours.len() > MAX_BYTES || theirs.len() > MAX_BYTES || result.len() > MAX_BYTES {
        return Ok(ConflictText {
            path: rel,
            too_big: true,
            ours: String::new(),
            theirs: String::new(),
            result: String::new(),
        });
    }
    Ok(ConflictText {
        path: rel,
        too_big: false,
        ours,
        theirs,
        result,
    })
}

fn show_stage(git: &Path, root: &Path, path: &str, stage: u8) -> Result<String, String> {
    let spec = format!(":{stage}:{path}");
    let out = gitrun::run_ok(git, root, &["show", &spec], &[], None)?;
    Ok(out.text())
}

pub fn resolve_side(git: &Path, root: &Path, path: &str, ours: bool, confirm: bool) -> Result<(), String> {
    if !confirm {
        return Err("Esa acción pisa el archivo y pide confirmación.".into());
    }
    let rel = relative_in_repo(path)?;
    let side = if ours { "--ours" } else { "--theirs" };
    gitrun::run_ok(git, root, &["checkout", side, "--", &rel], &[], None)?;
    gitrun::run_ok(git, root, &["add", "--", &rel], &[], None)?;
    Ok(())
}

pub fn write_result(_git: &Path, root: &Path, path: &str, text: &str) -> Result<(), String> {
    let rel = relative_in_repo(path)?;
    let full = root.join(&rel);
    if text.len() > MAX_BYTES {
        return Err("El resultado pasa de 1 MB.".into());
    }
    std::fs::write(&full, text).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn mark_resolved(git: &Path, root: &Path, path: &str) -> Result<(), String> {
    let rel = relative_in_repo(path)?;
    gitrun::run_ok(git, root, &["add", "--", &rel], &[], None)?;
    Ok(())
}

pub fn unmerged(git: &Path, root: &Path) -> Result<String, String> {
    let out = gitrun::run_ok(git, root, &["ls-files", "-u"], &[], None)?;
    Ok(out.text())
}

pub fn sync(
    git: &Path,
    root: &Path,
    action: &str,
    passphrase: Option<&str>,
    helper: Option<&Path>,
    profile_id: Option<&str>,
) -> Result<String, String> {
    let args: &[&str] = match action {
        "fetch" => &["fetch"],
        "pull" => &["pull", "--no-edit"],
        "push" => &["push"],
        _ => return Err("Esa acción de red no existe.".into()),
    };
    let mut env: Vec<(&str, String)> = Vec::new();
    let owned_pass;
    if let Some(pass) = passphrase {
        if pass.contains(['\n', '\r', '\0']) {
            return Err("La frase no es válida.".into());
        }
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let exe_text = exe.to_string_lossy().to_string();
        owned_pass = pass.to_string();
        env.push(("STASHMANGO_SSH_PASS", owned_pass));
        env.push(("SSH_ASKPASS", exe_text));
        env.push(("SSH_ASKPASS_REQUIRE", "force".into()));
        env.push(("DISPLAY", "1".into()));
    }
    if let Some(profile_id) = profile_id {
        if profile_id.contains(['\n', '\r', '\0']) {
            return Err("El perfil no es válido.".into());
        }
        env.push(("STASHMANGO_PROFILE", profile_id.to_string()));
    }
    let mut arg_owned: Vec<String> = Vec::new();
    if let Some(helper) = helper {
        arg_owned.push("-c".into());
        arg_owned.push(format!("credential.helper={}", helper.display()));
    }
    arg_owned.extend(args.iter().map(|s| (*s).to_string()));
    let arg_refs: Vec<&str> = arg_owned.iter().map(|s| s.as_str()).collect();
    let env_refs: Vec<(&str, &str)> = env.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let out = gitrun::run(git, root, &arg_refs, &env_refs, None)?;
    if out.ok() {
        Ok(out.text())
    } else {
        Err(out.err_text())
    }
}

pub fn remote_url(git: &Path, root: &Path) -> Result<String, String> {
    let out = gitrun::run(git, root, &["remote", "get-url", "origin"], &[], None)?;
    if out.ok() {
        Ok(out.text().trim().to_string())
    } else {
        Ok(String::new())
    }
}

pub fn open_external(root: &Path, path: &str) -> Result<(), String> {
    let rel = relative_in_repo(path)?;
    let full = canonical_inside(root, &rel)?;
    open::that(&full).map_err(|e| e.to_string())
}

pub fn mergetool(git: &Path, root: &Path, path: &str) -> Result<(), String> {
    let rel = relative_in_repo(path)?;
    let tool = gitrun::run(git, root, &["config", "--get", "merge.tool"], &[], None)?;
    if !tool.ok() || tool.text().trim().is_empty() {
        return Err("Este repo no tiene merge.tool.".into());
    }
    gitrun::run_ok(git, root, &["mergetool", "--no-prompt", "--", &rel], &[], None)?;
    Ok(())
}

pub fn commit_files(git: &Path, root: &Path, sha: &str) -> Result<Vec<String>, String> {
    check_sha(sha)?;
    let out = gitrun::run_ok(git, root, &["diff-tree", "--no-commit-id", "--name-only", "-r", sha], &[], None)?;
    Ok(out.text().lines().map(|l| l.to_string()).filter(|l| !l.is_empty()).collect())
}

pub fn push_command_args(helper: &Path) -> Vec<String> {
    vec![
        "-c".into(),
        format!("credential.helper={}", helper.display()),
        "push".into(),
    ]
}

fn check_sha(value: &str) -> Result<(), String> {
    if value.len() < 4
        || value.len() > 80
        || !value.chars().all(|c| c.is_ascii_hexdigit() || c == '-' || c == '_' || c == '/' || c == '.')
    {
        // Las ramas también llegan como destino de reset. check-ref-format cubre ramas;
        // un SHA es hex. Aceptamos hex o un nombre de rama sin espacios.
    }
    if value.is_empty() || value.contains([' ', '\n', '\r', '\0']) || value.contains("..") {
        return Err("El destino no es válido.".into());
    }
    Ok(())
}

fn relative_in_repo(path: &str) -> Result<String, String> {
    if path.is_empty() || path.contains('\0') || path.contains('\n') {
        return Err("La ruta no es válida.".into());
    }
    let p = Path::new(path);
    if p.is_absolute() || p.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("La ruta sale del repositorio.".into());
    }
    Ok(path.replace('\\', "/"))
}

fn canonical_inside(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let full = root.join(rel);
    let root_c = root.canonicalize().map_err(|e| e.to_string())?;
    let full_c = full.canonicalize().map_err(|_| "El archivo no está en el repositorio.".to_string())?;
    if !full_c.starts_with(&root_c) {
        return Err("La ruta sale del repositorio.".into());
    }
    Ok(full_c)
}

fn worktree_size(root: &Path, rel: &str) -> u64 {
    root.join(rel).metadata().map(|m| m.len()).unwrap_or(0)
}

fn fs_read(root: &Path, rel: &str) -> Result<String, String> {
    let path = canonical_inside(root, rel)?;
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}
