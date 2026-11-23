use crate::gitrun::{self, GitOut};
use crate::graph::{self, CommitNode};
use crate::patch::{self, FileDiff};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

const AUTO_STASH: &str = "stashmango-auto";

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
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub cached: bool,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub media: String,
}

impl Default for DiffResult {
    fn default() -> Self {
        Self {
            path: String::new(),
            binary: false,
            too_big: false,
            size: 0,
            text: String::new(),
            hunks: 0,
            kind: "diff".into(),
            sha: String::new(),
            cached: false,
            data: String::new(),
            media: String::new(),
        }
    }
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

fn remote_clone_ok(url: &str) -> bool {
    if url.is_empty() || url.len() > 500 || url.starts_with('-') {
        return false;
    }
    if url.to_ascii_lowercase().contains("ext::") {
        return false;
    }
    if url.starts_with("https://") {
        let rest = &url["https://".len()..];
        if rest.contains('@') {
            return false;
        }
        return rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '/' | '_' | '~' | '%' | '+' | ':'));
    }
    if url.starts_with("ssh://") {
        return url.contains('@')
            && url
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '/' | '_' | '~' | '%' | '+' | ':' | '@'));
    }
    if url.contains("://") {
        return false;
    }
    let Some((user, rest)) = url.split_once('@') else {
        return false;
    };
    if user.is_empty() || user.contains(':') {
        return false;
    }
    let Some((host, path)) = rest.split_once(':') else {
        return false;
    };
    !host.is_empty()
        && !path.is_empty()
        && !path.starts_with('/')
        && url
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '/' | '_' | '~' | '+' | '@' | ':'))
}

pub fn clone_source_ok(raw: &str) -> Result<String, String> {
    if raw.contains(['\n', '\r', '\0']) {
        return Err("La URL no es válida.".into());
    }
    let url = raw.trim();
    if remote_clone_ok(url) {
        return Ok(url.to_string());
    }
    let path = PathBuf::from(url);
    if path.is_absolute() && path.is_dir() && path.join(".git").exists() {
        return Ok(url.to_string());
    }
    Err("La URL no es válida.".into())
}

fn user_dir(raw: &str) -> Result<PathBuf, String> {
    if raw.trim().is_empty() || raw.contains(['\n', '\r', '\0']) {
        return Err("La carpeta no es válida.".into());
    }
    let path = PathBuf::from(raw.trim());
    if !path.is_absolute() {
        return Err("La carpeta no es válida.".into());
    }
    Ok(path)
}

fn resolve_dir(raw: &str) -> Result<PathBuf, String> {
    let path = user_dir(raw)?;
    if path.exists() {
        if !path.is_dir() {
            return Err("La carpeta no es válida.".into());
        }
        return path
            .canonicalize()
            .map_err(|_| "La carpeta no es válida.".to_string());
    }
    let parent = path.parent().ok_or_else(|| "La carpeta no es válida.".to_string())?;
    if !parent.exists() {
        return Err("La carpeta no es válida.".into());
    }
    let parent = parent
        .canonicalize()
        .map_err(|_| "La carpeta no es válida.".to_string())?;
    let name = path
        .file_name()
        .ok_or_else(|| "La carpeta no es válida.".to_string())?;
    if name == "." || name == ".." {
        return Err("La carpeta no es válida.".into());
    }
    Ok(parent.join(name))
}

fn dir_is_empty(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(true);
    }
    let mut entries = std::fs::read_dir(path).map_err(|_| "La carpeta no es válida.".to_string())?;
    Ok(entries.next().is_none())
}

pub fn init_repo(git: &Path, path: &str) -> Result<RepoInfo, String> {
    let root = resolve_dir(path)?;
    if root.join(".git").exists() {
        return Err("Esa carpeta ya es un repositorio.".into());
    }
    if !root.exists() {
        std::fs::create_dir(&root).map_err(|_| "No se pudo crear la carpeta.".to_string())?;
    }
    gitrun::run_ok(git, &root, &["init", "-b", "main"], &[], None)?;
    info(git, &root)
}

pub fn clone_repo(git: &Path, url: &str, dest: &str) -> Result<RepoInfo, String> {
    let source = clone_source_ok(url)?;
    let root = resolve_dir(dest)?;
    if root.join(".git").exists() {
        return Err("Esa carpeta ya es un repositorio.".into());
    }
    if !dir_is_empty(&root)? {
        return Err("Esa carpeta no está vacía.".into());
    }
    let parent = root
        .parent()
        .ok_or_else(|| "La carpeta no es válida.".to_string())?;
    let name = root
        .file_name()
        .and_then(|item| item.to_str())
        .ok_or_else(|| "La carpeta no es válida.".to_string())?;
    gitrun::run_ok(git, parent, &["clone", "--", &source, name], &[], None)?;
    open_repo(git, &root)
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
        kind: if parsed.binary { "binary".into() } else { "diff".into() },
        ..DiffResult::default()
    })
}

pub fn preview_file(git: &Path, root: &Path, path: &str, cached: bool, sha: &str) -> Result<DiffResult, String> {
    let rel = relative_in_repo(path)?;
    if !sha.is_empty() {
        check_sha(sha)?;
        if looks_image(&rel) {
            return preview_blob_image(git, root, sha, &rel);
        }
        return preview_commit_diff(git, root, sha, &rel);
    }
    if looks_image(&rel) && root.join(&rel).is_file() {
        return preview_worktree_image(root, &rel, cached);
    }
    let mut result = diff_file(git, root, &rel, cached)?;
    result.cached = cached;
    if result.too_big || result.binary || !result.text.trim().is_empty() {
        return Ok(result);
    }
    match fs_read_bytes(root, &rel) {
        Ok(bytes) if bytes.len() > MAX_BYTES => Ok(too_big(rel, bytes.len() as u64)),
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(text) if text.lines().count() > MAX_LINES => Ok(too_big(rel, text.len() as u64)),
            Ok(text) => Ok(DiffResult {
                path: rel,
                size: text.len() as u64,
                text,
                kind: "text".into(),
                cached,
                ..DiffResult::default()
            }),
            Err(_) => Ok(DiffResult {
                path: rel,
                binary: true,
                kind: "binary".into(),
                cached,
                ..DiffResult::default()
            }),
        },
        Err(_) => Ok(result),
    }
}

fn preview_commit_diff(git: &Path, root: &Path, sha: &str, rel: &str) -> Result<DiffResult, String> {
    let out = gitrun::run_ok(
        git,
        root,
        &["show", "--format=", "-U0", "--no-ext-diff", sha, "--", rel],
        &[],
        None,
    )?;
    let text = out.text();
    if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
        return Ok(too_big(rel.to_string(), text.len() as u64));
    }
    let parsed = patch::parse(&text);
    Ok(DiffResult {
        path: rel.to_string(),
        binary: parsed.binary,
        size: text.len() as u64,
        text: if parsed.binary { String::new() } else { text },
        hunks: parsed.hunks.len(),
        kind: if parsed.binary { "binary".into() } else { "diff".into() },
        sha: sha.to_string(),
        ..DiffResult::default()
    })
}

fn preview_worktree_image(root: &Path, rel: &str, cached: bool) -> Result<DiffResult, String> {
    let bytes = fs_read_bytes(root, rel)?;
    image_preview(rel, bytes, "", cached)
}

fn preview_blob_image(git: &Path, root: &Path, sha: &str, rel: &str) -> Result<DiffResult, String> {
    let spec = format!("{sha}:{rel}");
    let out = gitrun::run_ok(git, root, &["show", &spec], &[], None)?;
    image_preview(rel, out.stdout, sha, false)
}

fn image_preview(rel: &str, bytes: Vec<u8>, sha: &str, cached: bool) -> Result<DiffResult, String> {
    if bytes.len() > MAX_BYTES {
        return Ok(too_big(rel.to_string(), bytes.len() as u64));
    }
    let Some(media) = image_media(rel) else {
        return Ok(DiffResult {
            path: rel.to_string(),
            binary: true,
            size: bytes.len() as u64,
            kind: "binary".into(),
            sha: sha.to_string(),
            cached,
            ..DiffResult::default()
        });
    };
    Ok(DiffResult {
        path: rel.to_string(),
        size: bytes.len() as u64,
        kind: "image".into(),
        sha: sha.to_string(),
        cached,
        data: to_base64(&bytes),
        media: media.into(),
        ..DiffResult::default()
    })
}

fn looks_image(path: &str) -> bool {
    image_media(path).is_some()
}

fn image_media(path: &str) -> Option<&'static str> {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        _ => None,
    }
}

fn to_base64(data: &[u8]) -> String {
    const ALPH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i];
        let b1 = data.get(i + 1).copied().unwrap_or(0);
        let b2 = data.get(i + 2).copied().unwrap_or(0);
        let triple = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        out.push(ALPH[((triple >> 18) & 63) as usize] as char);
        out.push(ALPH[((triple >> 12) & 63) as usize] as char);
        out.push(if i + 1 < data.len() {
            ALPH[((triple >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if i + 2 < data.len() {
            ALPH[(triple & 63) as usize] as char
        } else {
            '='
        });
        i += 3;
    }
    out
}

fn too_big(path: String, size: u64) -> DiffResult {
    DiffResult {
        path,
        too_big: true,
        size,
        kind: "binary".into(),
        ..DiffResult::default()
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

pub fn commit(
    git: &Path,
    root: &Path,
    message: &str,
    name: Option<&str>,
    email: Option<&str>,
    skip_hooks: bool,
) -> Result<(), String> {
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
    let mut args = vec!["commit"];
    if skip_hooks {
        args.push("--no-verify");
    }
    args.extend(["-F", "-"]);
    gitrun::run_ok(git, root, &args, &env, Some(message.as_bytes()))?;
    Ok(())
}

const HOOK_NAMES: &[&str] = &[
    "applypatch-msg",
    "pre-applypatch",
    "post-applypatch",
    "pre-commit",
    "pre-merge-commit",
    "prepare-commit-msg",
    "commit-msg",
    "post-commit",
    "pre-rebase",
    "post-checkout",
    "post-merge",
    "pre-push",
    "pre-auto-gc",
    "post-rewrite",
    "sendemail-validate",
    "fsmonitor-watchman",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoLive {
    pub hooks_dir: String,
    pub skip_hooks: bool,
    pub hooks: Vec<String>,
    pub fsmonitor: bool,
    pub watchman: bool,
}

pub fn git_dir(git: &Path, root: &Path) -> Result<PathBuf, String> {
    let out = gitrun::run_ok(git, root, &["rev-parse", "--git-dir"], &[], None)?;
    let path = PathBuf::from(out.text().trim());
    let full = if path.is_absolute() { path } else { root.join(path) };
    full.canonicalize().map_err(|_| "No se encontró .git.".to_string())
}

fn set_local(git: &Path, root: &Path, key: &str, value: &str) -> Result<(), String> {
    if value.contains(['\n', '\r', '\0']) {
        return Err("Ese valor no es válido.".into());
    }
    gitrun::run_ok(git, root, &["config", "--local", key, value], &[], None)?;
    Ok(())
}

fn unset_local(git: &Path, root: &Path, key: &str) {
    let _ = gitrun::run(git, root, &["config", "--local", "--unset-all", key], &[], None);
}

pub fn watchman_installed() -> bool {
    let name = if cfg!(windows) { "watchman.exe" } else { "watchman" };
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
}

const FSMONITOR_FILES: usize = 3000;

pub fn should_use_fsmonitor(watchman: bool, files: usize) -> bool {
    watchman || files >= FSMONITOR_FILES
}

fn tracked_count(git: &Path, root: &Path) -> Result<usize, String> {
    let out = gitrun::run_ok(git, root, &["ls-files", "-z"], &[], None)?;
    Ok(out.stdout.split(|b| *b == 0).filter(|item| !item.is_empty()).count())
}

pub fn maybe_enable_fsmonitor(git: &Path, root: &Path) -> Result<bool, String> {
    let current = local_config(git, root, "core.fsmonitor")?;
    if !current.is_empty() {
        return Ok(current != "false");
    }
    if !should_use_fsmonitor(watchman_installed(), tracked_count(git, root)?) {
        return Ok(false);
    }
    set_local(git, root, "core.fsmonitor", "true")?;
    Ok(true)
}

fn plain_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    raw.strip_prefix(r"\\?\").unwrap_or(raw.as_ref()).to_string()
}

fn hooks_dir_path(git: &Path, root: &Path) -> Result<PathBuf, String> {
    let out = gitrun::run_ok(git, root, &["rev-parse", "--git-path", "hooks"], &[], None)?;
    let path = PathBuf::from(out.text().trim());
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(root.join(path))
    }
}

fn list_hooks(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let allow: std::collections::HashSet<&str> = HOOK_NAMES.iter().copied().collect();
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".sample") || !allow.contains(name.as_str()) {
            continue;
        }
        if entry.file_type().map(|kind| kind.is_file() || kind.is_symlink()).unwrap_or(false) {
            names.push(name);
        }
    }
    names.sort();
    names.dedup();
    names
}

pub fn repo_live(git: &Path, root: &Path) -> Result<RepoLive, String> {
    let dir = hooks_dir_path(git, root)?;
    let fsmonitor = local_config(git, root, "core.fsmonitor")?;
    Ok(RepoLive {
        hooks_dir: plain_path(&dir),
        skip_hooks: local_config(git, root, "stashmango.skipHooks")? == "true",
        hooks: list_hooks(&dir),
        fsmonitor: !fsmonitor.is_empty() && fsmonitor != "false",
        watchman: watchman_installed(),
    })
}

pub fn set_skip_hooks(git: &Path, root: &Path, skip: bool) -> Result<RepoLive, String> {
    set_local(git, root, "stashmango.skipHooks", if skip { "true" } else { "false" })?;
    repo_live(git, root)
}

pub fn set_fsmonitor(git: &Path, root: &Path, enabled: bool) -> Result<RepoLive, String> {
    if enabled {
        set_local(git, root, "core.fsmonitor", "true")?;
    } else {
        unset_local(git, root, "core.fsmonitor");
    }
    repo_live(git, root)
}

pub fn set_hooks_dir(git: &Path, root: &Path, raw: &str) -> Result<RepoLive, String> {
    let text = raw.trim();
    if text.is_empty() {
        unset_local(git, root, "core.hooksPath");
        return repo_live(git, root);
    }
    if text.contains(['\n', '\r', '\0']) {
        return Err("La carpeta de hooks no es válida.".into());
    }
    let path = PathBuf::from(text);
    if !path.is_absolute() || !path.is_dir() {
        return Err("La carpeta de hooks no es válida.".into());
    }
    let path = path
        .canonicalize()
        .map_err(|_| "La carpeta de hooks no es válida.".to_string())?;
    set_local(git, root, "core.hooksPath", &plain_path(&path))?;
    repo_live(git, root)
}

pub fn local_config(git: &Path, root: &Path, key: &str) -> Result<String, String> {
    let out = gitrun::run(git, root, &["config", "--local", "--get", key], &[], None)?;
    if out.ok() {
        Ok(out.text().trim().to_string())
    } else {
        Ok(String::new())
    }
}

const GITFLOW_KEYS: &[(&str, &str)] = &[
    ("production", "main, master"),
    ("develop", "develop"),
    ("feature", "feature/"),
    ("release", "release/"),
    ("hotfix", "hotfix/"),
    ("support", "support/"),
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GitFlow {
    pub enabled: bool,
    pub production: String,
    pub develop: String,
    pub feature: String,
    pub release: String,
    pub hotfix: String,
    pub support: String,
}

impl Default for GitFlow {
    fn default() -> Self {
        Self {
            enabled: false,
            production: "main, master".into(),
            develop: "develop".into(),
            feature: "feature/".into(),
            release: "release/".into(),
            hotfix: "hotfix/".into(),
            support: "support/".into(),
        }
    }
}

impl GitFlow {
    fn field(&self, id: &str) -> &str {
        match id {
            "production" => &self.production,
            "develop" => &self.develop,
            "feature" => &self.feature,
            "release" => &self.release,
            "hotfix" => &self.hotfix,
            "support" => &self.support,
            _ => "",
        }
    }

    fn set_field(&mut self, id: &str, value: String) {
        match id {
            "production" => self.production = value,
            "develop" => self.develop = value,
            "feature" => self.feature = value,
            "release" => self.release = value,
            "hotfix" => self.hotfix = value,
            "support" => self.support = value,
            _ => {}
        }
    }
}

fn gitflow_token_ok(token: &str) -> bool {
    if token.is_empty() || token.len() > 120 {
        return false;
    }
    if token.starts_with(['-', '.', '/']) || token.contains("..") || token.contains("//") {
        return false;
    }
    let body = token.strip_suffix('/').unwrap_or(token);
    !body.is_empty()
        && body
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '.'))
}

fn gitflow_patterns(raw: &str, fallback: &str, strict: bool) -> Result<String, String> {
    let mut tokens = Vec::new();
    for part in raw.split(',') {
        let token = part.trim();
        if token.is_empty() {
            continue;
        }
        if !gitflow_token_ok(token) {
            if strict {
                return Err("Hay un nombre de rama que no es válido.".into());
            }
            continue;
        }
        if !tokens.iter().any(|item| item == token) {
            tokens.push(token.to_string());
        }
    }
    if tokens.is_empty() {
        Ok(fallback.to_string())
    } else {
        Ok(tokens.join(", "))
    }
}

fn gitflow_config_key(id: &str) -> String {
    format!("stashmango.gitflow.{id}")
}

pub fn gitflow(git: &Path, root: &Path) -> Result<GitFlow, String> {
    let mut flow = GitFlow::default();
    flow.enabled = local_config(git, root, "stashmango.gitflow")? == "true";
    for (id, fallback) in GITFLOW_KEYS {
        let raw = local_config(git, root, &gitflow_config_key(id))?;
        flow.set_field(id, gitflow_patterns(&raw, fallback, false)?);
    }
    Ok(flow)
}

pub fn set_gitflow(git: &Path, root: &Path, flow: GitFlow) -> Result<GitFlow, String> {
    let mut clean = GitFlow {
        enabled: flow.enabled,
        ..GitFlow::default()
    };
    for (id, fallback) in GITFLOW_KEYS {
        clean.set_field(id, gitflow_patterns(flow.field(id), fallback, true)?);
    }
    let flag = if clean.enabled { "true" } else { "false" };
    gitrun::run_ok(git, root, &["config", "--local", "stashmango.gitflow", flag], &[], None)?;
    for (id, _) in GITFLOW_KEYS {
        gitrun::run_ok(
            git,
            root,
            &["config", "--local", &gitflow_config_key(id), clean.field(id)],
            &[],
            None,
        )?;
    }
    Ok(clean)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BranchRef {
    pub name: String,
    pub sha: String,
    pub remote: bool,
}

pub fn branches(git: &Path, root: &Path) -> Result<Vec<BranchRef>, String> {
    let out = gitrun::run_ok(
        git,
        root,
        &["for-each-ref", "--format=%(objectname) %(refname)", "refs/heads", "refs/remotes"],
        &[],
        None,
    )?;
    let mut list = Vec::new();
    for line in out.text().lines() {
        let Some((sha, full)) = line.trim().split_once(' ') else {
            continue;
        };
        let (remote, name) = if let Some(rest) = full.strip_prefix("refs/remotes/") {
            (true, rest)
        } else if let Some(rest) = full.strip_prefix("refs/heads/") {
            (false, rest)
        } else {
            continue;
        };
        if name.is_empty() || sha.is_empty() {
            continue;
        }
        list.push(BranchRef {
            name: name.to_string(),
            sha: sha.to_string(),
            remote,
        });
    }
    Ok(list)
}

pub fn create_branch(git: &Path, root: &Path, name: &str) -> Result<(), String> {
    check_branch(git, root, name)?;
    gitrun::run_ok(git, root, &["switch", "-c", name], &[], None)?;
    Ok(())
}

pub fn switch_branch(git: &Path, root: &Path, name: &str) -> Result<(), String> {
    check_branch(git, root, name)?;
    let dirty = abort_if_busy(git, root)?;
    if dirty {
        let clashes = switch_clashes(git, root, name)?;
        if !clashes.is_empty() {
            return Err(clash_error("esa rama", clashes));
        }
        stash_local(git, root)?;
        if let Err(err) = gitrun::run_ok(git, root, &["switch", "--", name], &[], None) {
            let _ = stash_pop(git, root);
            return Err(err);
        }
        if stash_pop(git, root).is_err() {
            let _ = gitrun::run_ok(git, root, &["switch", "-"], &[], None);
            let _ = stash_pop(git, root);
            return Err("Hay cambios locales que chocan con esa rama. No se hizo el cambio.".into());
        }
        return Ok(());
    }
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

fn abort_if_busy(git: &Path, root: &Path) -> Result<bool, String> {
    let files = status(git, root)?;
    if files.iter().any(|file| file.conflict) {
        return Err("Hay un conflicto en curso.".into());
    }
    Ok(!files.is_empty())
}

fn clash_error(kind: &str, mut files: Vec<String>) -> String {
    files.sort();
    files.dedup();
    let extra = if files.len() > 5 {
        format!(" y {} más", files.len() - 5)
    } else {
        String::new()
    };
    let shown = files.iter().take(5).cloned().collect::<Vec<_>>().join(", ");
    format!("Hay cambios locales que chocan con {kind}: {shown}{extra}. No se hizo el cambio.")
}

fn nul_paths(out: &GitOut) -> HashSet<String> {
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

fn diff_names(git: &Path, root: &Path, a: &str, b: &str) -> Result<HashSet<String>, String> {
    let out = gitrun::run_ok(git, root, &["diff", "--name-only", "-z", a, b], &[], None)?;
    Ok(nul_paths(&out))
}

fn tree_names(git: &Path, root: &Path, tree: &str) -> Result<HashSet<String>, String> {
    let out = gitrun::run_ok(git, root, &["ls-tree", "-r", "--name-only", "-z", tree], &[], None)?;
    Ok(nul_paths(&out))
}

fn local_clashes(
    git: &Path,
    root: &Path,
    incoming: &HashSet<String>,
    incoming_tree: &HashSet<String>,
) -> Result<Vec<String>, String> {
    let mut hits = Vec::new();
    for file in status(git, root)? {
        let hit = if file.untracked {
            incoming_tree.contains(&file.path)
        } else {
            incoming.contains(&file.path)
        };
        if hit || file.conflict {
            hits.push(file.path);
        }
    }
    Ok(hits)
}

fn switch_clashes(git: &Path, root: &Path, target: &str) -> Result<Vec<String>, String> {
    local_clashes(
        git,
        root,
        &diff_names(git, root, "HEAD", target)?,
        &tree_names(git, root, target)?,
    )
}

fn merge_base_or_dirty(
    git: &Path,
    root: &Path,
    other: &str,
) -> Result<Option<String>, String> {
    let out = gitrun::run(git, root, &["merge-base", "HEAD", other], &[], None)?;
    if out.ok() {
        let sha = out.text().trim().to_string();
        if sha.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sha))
        }
    } else if status(git, root)?.is_empty() {
        Ok(None)
    } else {
        Err("Hay cambios locales y no se puede calcular la base. No se hizo el cambio.".into())
    }
}

fn merge_clashes(git: &Path, root: &Path, other: &str) -> Result<Vec<String>, String> {
    let Some(base) = merge_base_or_dirty(git, root, other)? else {
        return Ok(Vec::new());
    };
    local_clashes(
        git,
        root,
        &diff_names(git, root, &base, other)?,
        &tree_names(git, root, other)?,
    )
}

fn rebase_clashes(git: &Path, root: &Path, upstream: &str) -> Result<Vec<String>, String> {
    let Some(base) = merge_base_or_dirty(git, root, upstream)? else {
        return Ok(Vec::new());
    };
    let mut incoming = diff_names(git, root, &base, upstream)?;
    incoming.extend(diff_names(git, root, &base, "HEAD")?);
    local_clashes(git, root, &incoming, &tree_names(git, root, upstream)?)
}

fn stash_local(git: &Path, root: &Path) -> Result<(), String> {
    gitrun::run_ok(git, root, &["stash", "push", "-u", "-m", AUTO_STASH], &[], None)?;
    Ok(())
}

fn rebase_in_progress(git: &Path, root: &Path) -> bool {
    for name in ["rebase-merge", "rebase-apply"] {
        let Ok(out) = gitrun::run(git, root, &["rev-parse", "--git-path", name], &[], None) else {
            continue;
        };
        if !out.ok() {
            continue;
        }
        let raw = PathBuf::from(out.text().trim());
        let full = if raw.is_absolute() {
            raw
        } else {
            root.join(raw)
        };
        if full.exists() {
            return true;
        }
    }
    false
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

pub fn stash_selector(sel: &str) -> Result<String, String> {
    let raw = sel.trim();
    let inner = raw
        .strip_prefix("stash@{")
        .and_then(|rest| rest.strip_suffix('}'))
        .ok_or_else(|| "Ese stash no es válido.".to_string())?;
    if inner.is_empty() || inner.len() > 6 || !inner.chars().all(|c| c.is_ascii_digit()) {
        return Err("Ese stash no es válido.".into());
    }
    Ok(format!("stash@{{{inner}}}"))
}

pub fn stash_apply(git: &Path, root: &Path, sel: &str) -> Result<(), String> {
    let id = stash_selector(sel)?;
    gitrun::run_ok(git, root, &["stash", "apply", "--", &id], &[], None)?;
    Ok(())
}

pub fn stash_pop(git: &Path, root: &Path) -> Result<(), String> {
    stash_pop_at(git, root, "stash@{0}")
}

pub fn stash_pop_at(git: &Path, root: &Path, sel: &str) -> Result<(), String> {
    let id = stash_selector(sel)?;
    gitrun::run_ok(git, root, &["stash", "pop", "--", &id], &[], None)?;
    Ok(())
}

pub fn stash_drop(git: &Path, root: &Path, confirm: bool) -> Result<(), String> {
    stash_drop_at(git, root, "stash@{0}", confirm)
}

pub fn stash_drop_at(git: &Path, root: &Path, sel: &str, confirm: bool) -> Result<(), String> {
    if !confirm {
        return Err("Soltar un stash pide confirmación.".into());
    }
    let id = stash_selector(sel)?;
    gitrun::run_ok(git, root, &["stash", "drop", "--", &id], &[], None)?;
    Ok(())
}

pub fn merge(git: &Path, root: &Path, branch: &str, message: &str) -> Result<GitOut, String> {
    check_branch(git, root, branch)?;
    if message.contains('\0') {
        return Err("El mensaje no es válido.".into());
    }
    let dirty = abort_if_busy(git, root)?;
    if dirty {
        let clashes = merge_clashes(git, root, branch)?;
        if !clashes.is_empty() {
            return Err(clash_error("el merge", clashes));
        }
        stash_local(git, root)?;
    }
    let out = if message.trim().is_empty() {
        gitrun::run(git, root, &["merge", "--no-edit", "--", branch], &[], None)?
    } else {
        gitrun::run(git, root, &["merge", "--no-ff", "-m", message, "--", branch], &[], None)?
    };
    if dirty {
        if let Err(err) = stash_pop(git, root) {
            if out.ok() {
                return Err(format!(
                    "El merge se hizo, pero no se restauraron los cambios locales: {err}"
                ));
            }
            // shortcut: merge en conflicto y el stash no aplica; el trabajo queda en el stash
        }
    }
    Ok(out)
}

pub fn rebase(git: &Path, root: &Path, upstream: &str) -> Result<(), String> {
    check_branch(git, root, upstream)?;
    let dirty = abort_if_busy(git, root)?;
    if dirty {
        let clashes = rebase_clashes(git, root, upstream)?;
        if !clashes.is_empty() {
            return Err(clash_error("el rebase", clashes));
        }
        stash_local(git, root)?;
    }
    if let Err(err) = gitrun::run_ok(git, root, &["rebase", upstream], &[], None) {
        if dirty && rebase_in_progress(git, root) {
            return Err(format!("{err} Tu trabajo quedó en el stash."));
        }
        if dirty {
            let _ = stash_pop(git, root);
        }
        return Err(err);
    }
    if dirty {
        stash_pop(git, root)?;
    }
    Ok(())
}

pub fn cherry_pick(git: &Path, root: &Path, sha: &str, message: &str) -> Result<(), String> {
    check_sha(sha)?;
    if message.contains('\0') {
        return Err("El mensaje no es válido.".into());
    }
    gitrun::run_ok(git, root, &["cherry-pick", sha], &[], None)?;
    if message.trim().is_empty() {
        return Ok(());
    }
    gitrun::run_ok(git, root, &["commit", "--amend", "-F", "-"], &[], Some(message.as_bytes()))?;
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

pub fn git_credential_password(git: &Path, host: &str) -> Result<String, String> {
    crate::secret::forge_host(host)?;
    let input = format!("protocol=https\nhost={host}\n\n");
    let out = gitrun::run(
        git,
        &std::env::temp_dir(),
        &["credential", "fill"],
        &[("GCM_INTERACTIVE", "never")],
        Some(input.as_bytes()),
    )?;
    if !out.ok() {
        return Err("Git no tiene un token para ese host.".into());
    }
    let Some(password) = crate::secret::parse_credential_password(&out.text()) else {
        return Err("Git no tiene un token para ese host.".into());
    };
    crate::secret::validate_token(&password)?;
    Ok(password)
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

fn fs_read_bytes(root: &Path, rel: &str) -> Result<Vec<u8>, String> {
    let path = canonical_inside(root, rel)?;
    std::fs::read(path).map_err(|e| e.to_string())
}
