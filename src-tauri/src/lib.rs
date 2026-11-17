pub mod gitrun;
pub mod graph;
pub mod ops;
pub mod patch;
pub mod pathcfg;
pub mod prs;
pub mod secret;
pub mod store;

use ops::ReplayStep;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

struct AppState {
    git: gitrun::Git,
    repo: Mutex<Option<PathBuf>>,
    undo: Mutex<HashMap<PathBuf, String>>,
}

fn store_undo(map: &mut HashMap<PathBuf, String>, root: &Path, sha: String) {
    map.insert(root.to_path_buf(), sha);
}

fn take_undo(map: &mut HashMap<PathBuf, String>, root: &Path) -> Option<String> {
    map.remove(root)
}

#[derive(Deserialize)]
struct LinePick {
    hunk: usize,
    line: usize,
}

fn repo_path(state: &AppState) -> Result<PathBuf, String> {
    state
        .repo
        .lock()
        .map_err(|_| "El estado de la app está ocupado.".to_string())?
        .clone()
        .ok_or_else(|| "Abre un repositorio primero.".to_string())
}

fn identity(root: &std::path::Path) -> Result<(Option<String>, Option<String>), String> {
    match store::profile_for(root)? {
        Some(profile) if !profile.name.is_empty() && !profile.email.is_empty() => {
            Ok((Some(profile.name), Some(profile.email)))
        }
        _ => Ok((None, None)),
    }
}

fn profile_id_for(root: &std::path::Path) -> Result<String, String> {
    Ok(store::profile_for(root)?
        .map(|profile| profile.id)
        .unwrap_or_else(|| store::DEFAULT_ID.to_string()))
}

#[tauri::command]
fn open_repo(state: State<AppState>, path: String) -> Result<ops::RepoInfo, String> {
    let info = ops::open_repo(&state.git.bin, PathBuf::from(&path).as_path())?;
    let root = PathBuf::from(&info.path);
    store::remember(&root, &info.branch)?;
    *state.repo.lock().map_err(|e| e.to_string())? = Some(root);
    Ok(info)
}

#[tauri::command]
fn recents() -> Result<Vec<store::Recent>, String> {
    store::recents()
}

#[tauri::command]
fn layout() -> Result<store::Columns, String> {
    store::layout()
}

#[tauri::command]
fn save_layout(layout: store::Columns) -> Result<(), String> {
    store::save_layout(&layout)
}

#[tauri::command]
fn templates() -> Result<store::Templates, String> {
    store::templates()
}

#[tauri::command]
fn save_templates(templates: store::Templates) -> Result<store::Templates, String> {
    store::save_templates(&templates)
}

#[tauri::command]
fn refresh(state: State<AppState>) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    let info = ops::info(&state.git.bin, &root)?;
    store::remember(&root, &info.branch)?;
    Ok(info)
}

#[tauri::command]
fn log_page(state: State<AppState>, skip: usize, open_lanes: Vec<Option<String>>) -> Result<ops::LogPage, String> {
    let root = repo_path(&state)?;
    ops::log_page(&state.git.bin, &root, skip, open_lanes)
}

#[tauri::command]
fn diff_file(state: State<AppState>, path: String, cached: bool, sha: Option<String>) -> Result<ops::DiffResult, String> {
    let root = repo_path(&state)?;
    ops::preview_file(&state.git.bin, &root, &path, cached, sha.as_deref().unwrap_or(""))
}

#[tauri::command]
fn stage_file(state: State<AppState>, path: String, stage: bool) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::stage_file(&state.git.bin, &root, &path, stage)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn apply_selection(
    state: State<AppState>,
    path: String,
    cached: bool,
    hunks: Vec<usize>,
    lines: Vec<LinePick>,
) -> Result<(), String> {
    let root = repo_path(&state)?;
    let pairs: Vec<(usize, usize)> = lines.into_iter().map(|l| (l.hunk, l.line)).collect();
    ops::apply_selection(&state.git.bin, &root, &path, cached, &hunks, &pairs)
}

#[tauri::command]
fn commit(state: State<AppState>, message: String) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    let (name, email) = identity(&root)?;
    ops::commit(&state.git.bin, &root, &message, name.as_deref(), email.as_deref())?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn branches(state: State<AppState>) -> Result<Vec<ops::BranchRef>, String> {
    let root = repo_path(&state)?;
    ops::branches(&state.git.bin, &root)
}

#[tauri::command]
fn create_branch(state: State<AppState>, name: String) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::create_branch(&state.git.bin, &root, &name)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn switch_branch(state: State<AppState>, name: String) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::switch_branch(&state.git.bin, &root, &name)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn worktrees(state: State<AppState>) -> Result<Vec<ops::Worktree>, String> {
    let root = repo_path(&state)?;
    ops::worktrees(&state.git.bin, &root)
}

#[tauri::command]
fn add_worktree(state: State<AppState>, path: String, branch: String) -> Result<Vec<ops::Worktree>, String> {
    let root = repo_path(&state)?;
    ops::add_worktree(&state.git.bin, &root, PathBuf::from(path).as_path(), &branch)?;
    ops::worktrees(&state.git.bin, &root)
}

#[tauri::command]
fn remove_worktree(state: State<AppState>, path: String, confirm: bool) -> Result<Vec<ops::Worktree>, String> {
    let root = repo_path(&state)?;
    ops::remove_worktree(&state.git.bin, &root, PathBuf::from(path).as_path(), confirm)?;
    ops::worktrees(&state.git.bin, &root)
}

#[tauri::command]
fn stash_list(state: State<AppState>) -> Result<Vec<String>, String> {
    let root = repo_path(&state)?;
    ops::stash_list(&state.git.bin, &root)
}

#[tauri::command]
fn stash_push(state: State<AppState>, message: String) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::stash_push(&state.git.bin, &root, &message)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn stash_pop(state: State<AppState>) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::stash_pop(&state.git.bin, &root)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn stash_drop(state: State<AppState>, confirm: bool) -> Result<Vec<String>, String> {
    let root = repo_path(&state)?;
    ops::stash_drop(&state.git.bin, &root, confirm)?;
    ops::stash_list(&state.git.bin, &root)
}

#[tauri::command]
fn merge(state: State<AppState>, branch: String, message: Option<String>) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    let out = ops::merge(&state.git.bin, &root, &branch, message.as_deref().unwrap_or(""))?;
    if !out.ok() && !out.err_text().to_lowercase().contains("conflict") && out.code != 1 {
        return Err(out.err_text());
    }
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn rebase_branch(state: State<AppState>, upstream: String) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::rebase(&state.git.bin, &root, &upstream)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn replay(state: State<AppState>, base: String, steps: Vec<ReplayStep>) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::replay(&state.git.bin, &root, &base, &steps)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn cherry_pick(state: State<AppState>, sha: String, message: Option<String>) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::cherry_pick(&state.git.bin, &root, &sha, message.as_deref().unwrap_or(""))?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn reset_hard(state: State<AppState>, target: String, confirm: bool) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    let previous = ops::reset_hard(&state.git.bin, &root, &target, confirm)?;
    let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
    store_undo(&mut undo, &root, previous);
    drop(undo);
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn undo_reset(state: State<AppState>) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    let mut undo = state.undo.lock().map_err(|e| e.to_string())?;
    let previous = take_undo(&mut undo, &root).ok_or("No hay un commit anterior en esta sesión.")?;
    if let Err(err) = ops::reset_hard(&state.git.bin, &root, &previous, true) {
        store_undo(&mut undo, &root, previous);
        return Err(err);
    }
    drop(undo);
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn conflict_text(state: State<AppState>, path: String) -> Result<ops::ConflictText, String> {
    let root = repo_path(&state)?;
    ops::conflict_text(&state.git.bin, &root, &path)
}

#[tauri::command]
fn resolve_side(state: State<AppState>, path: String, ours: bool, confirm: bool) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::resolve_side(&state.git.bin, &root, &path, ours, confirm)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn write_result(state: State<AppState>, path: String, text: String) -> Result<(), String> {
    let root = repo_path(&state)?;
    ops::write_result(&state.git.bin, &root, &path, &text)
}

#[tauri::command]
fn mark_resolved(state: State<AppState>, path: String) -> Result<ops::RepoInfo, String> {
    let root = repo_path(&state)?;
    ops::mark_resolved(&state.git.bin, &root, &path)?;
    ops::info(&state.git.bin, &root)
}

#[tauri::command]
fn network(state: State<AppState>, action: String, passphrase: Option<String>) -> Result<String, String> {
    let root = repo_path(&state)?;
    let url = ops::remote_url(&state.git.bin, &root)?;
    let host = prs::parse_remote(&url).map(|remote| remote.host);
    let profile_id = profile_id_for(&root)?;
    let helper = if host.as_deref().is_some_and(|item| secret::has_token(&profile_id, item)) {
        std::env::current_exe().ok()
    } else {
        None
    };
    ops::sync(
        &state.git.bin,
        &root,
        &action,
        passphrase.as_deref(),
        helper.as_deref(),
        Some(&profile_id),
    )
}

#[tauri::command]
fn remote(state: State<AppState>) -> Result<String, String> {
    let root = repo_path(&state)?;
    ops::remote_url(&state.git.bin, &root)
}

#[tauri::command]
fn save_token(profile_id: String, host: String, token: String) -> Result<(), String> {
    secret::save_token(&profile_id, &host, &token)
}

#[tauri::command]
fn has_token(profile_id: String, host: String) -> Result<bool, String> {
    Ok(secret::has_token(&profile_id, &host))
}

#[tauri::command]
fn clear_token(profile_id: String, host: String) -> Result<(), String> {
    secret::clear_token(&profile_id, &host)
}

fn token_for(root: &std::path::Path, host: &str) -> Result<String, String> {
    let profile_id = profile_id_for(root)?;
    secret::read_token(&profile_id, host)?.ok_or("Falta un token de este perfil para ese host.".into())
}

#[tauri::command]
fn list_prs(state: State<AppState>) -> Result<Vec<prs::PullRequest>, String> {
    let root = repo_path(&state)?;
    let url = ops::remote_url(&state.git.bin, &root)?;
    let remote = prs::parse_remote(&url).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    let token = token_for(&root, &remote.host)?;
    prs::list_prs(&url, &token)
}

#[tauri::command]
fn create_pr(state: State<AppState>, title: String, head: String, base: String) -> Result<prs::PullRequest, String> {
    let root = repo_path(&state)?;
    let url = ops::remote_url(&state.git.bin, &root)?;
    let remote = prs::parse_remote(&url).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    let token = token_for(&root, &remote.host)?;
    prs::create_pr(&url, &token, &title, &head, &base)
}

#[tauri::command]
fn merge_pr(state: State<AppState>, number: u64) -> Result<(), String> {
    let root = repo_path(&state)?;
    let url = ops::remote_url(&state.git.bin, &root)?;
    let remote = prs::parse_remote(&url).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    let token = token_for(&root, &remote.host)?;
    prs::merge_pr(&url, &token, number)
}

#[tauri::command]
fn pr_diff(state: State<AppState>, number: u64) -> Result<String, String> {
    let root = repo_path(&state)?;
    let url = ops::remote_url(&state.git.bin, &root)?;
    let remote = prs::parse_remote(&url).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    let token = token_for(&root, &remote.host)?;
    prs::pr_diff(&url, &token, number)
}

#[tauri::command]
fn profiles() -> Result<store::ProfilesFile, String> {
    store::profiles()
}

#[tauri::command]
fn save_profile(profile: store::Profile) -> Result<store::ProfilesFile, String> {
    store::save_profile(profile)
}

#[tauri::command]
fn delete_profile(id: String) -> Result<store::ProfilesFile, String> {
    store::delete_profile(&id)
}

#[tauri::command]
fn set_repo_profile(state: State<AppState>, profile_id: Option<String>) -> Result<(), String> {
    let root = repo_path(&state)?;
    store::set_repo_profile(&root, profile_id.as_deref())
}

#[tauri::command]
fn active_profile(state: State<AppState>) -> Result<Option<store::Profile>, String> {
    let root = repo_path(&state)?;
    store::profile_for(&root)
}

#[tauri::command]
fn open_external(state: State<AppState>, path: String) -> Result<(), String> {
    let root = repo_path(&state)?;
    ops::open_external(&root, &path)
}

#[tauri::command]
fn mergetool(state: State<AppState>, path: String) -> Result<(), String> {
    let root = repo_path(&state)?;
    ops::mergetool(&state.git.bin, &root, &path)
}

#[tauri::command]
fn commit_files(state: State<AppState>, sha: String) -> Result<Vec<String>, String> {
    let root = repo_path(&state)?;
    ops::commit_files(&state.git.bin, &root, &sha)
}

#[tauri::command]
fn set_external_git(enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let dir = gitrun::bundled_dir().ok_or("No hay Git portable en esta compilación.")?;
        return pathcfg::set_user_path(&dir, enabled);
    }
    #[cfg(not(windows))]
    {
        let _ = enabled;
        let home = std::env::var("HOME").map_err(|_| "No hay carpeta de usuario.".to_string())?;
        let local = std::path::PathBuf::from(home).join(".local/bin");
        let path = std::env::var("PATH").unwrap_or_default();
        if !pathcfg::local_bin_on_path(&path, &local.to_string_lossy()) {
            return Err("Agrega ~/.local/bin al PATH y reinicia. StashMango no reescribe el archivo de la shell.".into());
        }
        let link = local.join("git");
        if enabled {
            let bin = gitrun::bundled_bin().ok_or("No hay Git portable en esta compilación.")?;
            if link.exists() {
                std::fs::remove_file(&link).map_err(|e| e.to_string())?;
            }
            std::os::unix::fs::symlink(&bin, &link).map_err(|e| e.to_string())?;
        } else if link.symlink_metadata().is_ok() {
            std::fs::remove_file(&link).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub fn credential(action: &str) {
    if action != "get" {
        return;
    }
    use std::io::Read;
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    if let Ok(answer) = secret::credential_get(&input) {
        print!("{answer}");
    }
}

#[cfg(test)]
mod tests {
    use super::{store_undo, take_undo};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn undo_stays_with_its_repo() {
        let mut map = HashMap::new();
        let first = PathBuf::from("uno");
        let second = PathBuf::from("dos");
        store_undo(&mut map, &first, "aaa".into());
        store_undo(&mut map, &second, "bbb".into());
        assert_eq!(take_undo(&mut map, &first).as_deref(), Some("aaa"));
        assert_eq!(map.get(&second).map(String::as_str), Some("bbb"));
        assert!(take_undo(&mut map, &first).is_none());
    }
}

pub fn askpass() {
    if let Ok(pass) = std::env::var("STASHMANGO_SSH_PASS") {
        print!("{pass}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            git: gitrun::resolve(),
            repo: Mutex::new(None),
            undo: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            open_repo,
            recents,
            layout,
            save_layout,
            templates,
            save_templates,
            refresh,
            log_page,
            diff_file,
            stage_file,
            apply_selection,
            commit,
            branches,
            create_branch,
            switch_branch,
            worktrees,
            add_worktree,
            remove_worktree,
            stash_list,
            stash_push,
            stash_pop,
            stash_drop,
            merge,
            rebase_branch,
            replay,
            cherry_pick,
            reset_hard,
            undo_reset,
            conflict_text,
            resolve_side,
            write_result,
            mark_resolved,
            network,
            remote,
            save_token,
            has_token,
            clear_token,
            list_prs,
            create_pr,
            merge_pr,
            pr_diff,
            profiles,
            save_profile,
            delete_profile,
            set_repo_profile,
            active_profile,
            open_external,
            mergetool,
            commit_files,
            set_external_git
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
