use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recent {
    pub path: String,
    pub name: String,
    pub branch: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_true")]
    pub show_toolbar_labels: bool,
    #[serde(default)]
    pub show_author_initials: bool,
    #[serde(default = "default_true")]
    pub show_ghost_branch: bool,
    #[serde(default = "default_true")]
    pub highlight_branch_rows: bool,
    #[serde(default = "default_true")]
    pub auto_group_worktrees: bool,
    #[serde(default = "default_true")]
    pub show_spell_check: bool,
    #[serde(default = "default_graph_branch_visibility")]
    pub graph_branch_visibility: String,
    #[serde(default = "default_true")]
    pub show_branches_in_graph: bool,
    #[serde(default = "default_true")]
    pub show_tags_in_graph: bool,
    #[serde(default = "default_true")]
    pub show_author_in_graph: bool,
    #[serde(default = "default_true")]
    pub show_date_in_graph: bool,
    #[serde(default = "default_true")]
    pub show_message_in_graph: bool,
    #[serde(default = "default_description_visibility")]
    pub show_description_in_graph: String,
    #[serde(default)]
    pub show_sha_in_graph: bool,
    #[serde(default)]
    pub show_tree_in_graph: bool,
    #[serde(default)]
    pub graph_color_index: u32,
    #[serde(default = "default_group_color")]
    pub default_group_color: String,
}

fn default_graph_branch_visibility() -> String {
    "all".into()
}

fn default_description_visibility() -> String {
    "always".into()
}

fn default_group_color() -> String {
    "gray".into()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_toolbar_labels: true,
            show_author_initials: false,
            show_ghost_branch: true,
            highlight_branch_rows: true,
            auto_group_worktrees: true,
            show_spell_check: true,
            graph_branch_visibility: default_graph_branch_visibility(),
            show_branches_in_graph: true,
            show_tags_in_graph: true,
            show_author_in_graph: true,
            show_date_in_graph: true,
            show_message_in_graph: true,
            show_description_in_graph: default_description_visibility(),
            show_sha_in_graph: false,
            show_tree_in_graph: false,
            graph_color_index: 0,
            default_group_color: default_group_color(),
        }
    }
}

pub const DEFAULT_ID: &str = "default";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub name: String,
    pub email: String,
}

pub fn default_profile() -> Profile {
    Profile {
        id: DEFAULT_ID.into(),
        label: "default".into(),
        name: String::new(),
        email: String::new(),
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProfilesFile {
    pub profiles: Vec<Profile>,
    pub repos: Vec<RepoProfile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepoProfile {
    pub path: String,
    pub profile_id: String,
}

pub fn data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("STASHMANGO_HOME") {
        return PathBuf::from(home);
    }
    #[cfg(windows)]
    {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        return PathBuf::from(base).join("StashMango");
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        return PathBuf::from(home).join("Library/Application Support/StashMango");
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
            return PathBuf::from(xdg).join("StashMango");
        }
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".local/share/StashMango")
    }
}

fn ensure() -> Result<PathBuf, String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn recents() -> Result<Vec<Recent>, String> {
    let path = data_dir().join("recents.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub fn remember(path: &Path, branch: &str) -> Result<(), String> {
    let dir = ensure()?;
    let mut list = recents()?;
    let path_text = path.to_string_lossy().to_string();
    list.retain(|item| item.path != path_text);
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path_text.clone());
    list.insert(
        0,
        Recent {
            path: path_text,
            name,
            branch: branch.to_string(),
        },
    );
    list.truncate(20);
    let json = serde_json::to_string_pretty(&list).map_err(|e| e.to_string())?;
    fs::write(dir.join("recents.json"), json).map_err(|e| e.to_string())
}

pub fn profiles() -> Result<ProfilesFile, String> {
    let path = data_dir().join("profiles.json");
    let mut file = if path.exists() {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text).unwrap_or_default()
    } else {
        ProfilesFile::default()
    };
    if !file.profiles.iter().any(|item| item.id == DEFAULT_ID) {
        file.profiles.insert(0, default_profile());
        save_profiles(&file)?;
    }
    Ok(file)
}

fn save_profiles(file: &ProfilesFile) -> Result<(), String> {
    let dir = ensure()?;
    let json = serde_json::to_string_pretty(file).map_err(|e| e.to_string())?;
    fs::write(dir.join("profiles.json"), json).map_err(|e| e.to_string())
}

pub fn validate_identity(name: &str, email: &str) -> Result<(), String> {
    if name.is_empty() && email.is_empty() {
        return Ok(());
    }
    if name.is_empty() || name.len() > 200 || name.contains(['\n', '\r']) {
        return Err("El nombre del perfil no es válido.".into());
    }
    if email.contains(['\n', '\r', ' ']) || email.matches('@').count() != 1 {
        return Err("El correo del perfil no es válido.".into());
    }
    let (local, host) = email.split_once('@').unwrap();
    if local.is_empty() || host.is_empty() {
        return Err("El correo del perfil no es válido.".into());
    }
    Ok(())
}

pub fn save_profile(profile: Profile) -> Result<ProfilesFile, String> {
    validate_identity(&profile.name, &profile.email)?;
    if profile.label.is_empty() || profile.label.contains(['\n', '\r']) {
        return Err("La etiqueta del perfil no es válida.".into());
    }
    let mut file = profiles()?;
    if let Some(existing) = file.profiles.iter_mut().find(|item| item.id == profile.id) {
        *existing = profile;
    } else {
        file.profiles.push(profile);
    }
    save_profiles(&file)?;
    Ok(file)
}

pub fn delete_profile(id: &str) -> Result<ProfilesFile, String> {
    if id == DEFAULT_ID {
        return Err("El perfil default no se puede borrar.".into());
    }
    let mut file = profiles()?;
    file.profiles.retain(|item| item.id != id);
    file.repos.retain(|item| item.profile_id != id);
    save_profiles(&file)?;
    Ok(file)
}

pub fn set_repo_profile(path: &Path, profile_id: Option<&str>) -> Result<(), String> {
    let mut file = profiles()?;
    let path_text = path.to_string_lossy().to_string();
    file.repos.retain(|item| item.path != path_text);
    let id = profile_id.unwrap_or("");
    if !id.is_empty() && !file.profiles.iter().any(|item| item.id == id) {
        return Err("Ese perfil no existe.".into());
    }
    file.repos.push(RepoProfile {
        path: path_text,
        profile_id: id.to_string(),
    });
    save_profiles(&file)
}

fn default_true() -> bool {
    true
}
fn default_side_width() -> u32 {
    268
}
fn default_detail_width() -> u32 {
    360
}
fn default_refs_width() -> u32 {
    220
}
fn default_graph_width() -> u32 {
    160
}
fn default_graph_columns() -> Vec<String> {
    ["refs", "graph", "message", "date"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn keep_graph_columns(columns: &[String]) -> Vec<String> {
    const ORDER: [&str; 6] = ["refs", "graph", "message", "author", "date", "hash"];
    let picked: Vec<String> = ORDER
        .iter()
        .filter(|id| columns.iter().any(|item| item == *id))
        .map(|id| (*id).to_string())
        .collect();
    if picked.is_empty() {
        default_graph_columns()
    } else {
        picked
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Columns {
    #[serde(default = "default_true")]
    pub side: bool,
    #[serde(default = "default_true")]
    pub detail: bool,
    #[serde(default = "default_side_width")]
    pub side_width: u32,
    #[serde(default = "default_detail_width")]
    pub detail_width: u32,
    #[serde(default = "default_refs_width")]
    pub refs_width: u32,
    #[serde(default = "default_graph_width")]
    pub graph_width: u32,
    #[serde(default = "default_graph_columns")]
    pub graph_columns: Vec<String>,
}

impl Default for Columns {
    fn default() -> Self {
        Self {
            side: true,
            detail: true,
            side_width: default_side_width(),
            detail_width: default_detail_width(),
            refs_width: default_refs_width(),
            graph_width: default_graph_width(),
            graph_columns: default_graph_columns(),
        }
    }
}

pub fn layout() -> Result<Columns, String> {
    let path = data_dir().join("layout.json");
    if !path.exists() {
        return Ok(Columns::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub fn save_layout(columns: &Columns) -> Result<(), String> {
    let dir = ensure()?;
    let columns = Columns {
        side: columns.side,
        detail: columns.detail,
        side_width: columns.side_width.clamp(180, 480),
        detail_width: columns.detail_width.clamp(240, 560),
        refs_width: columns.refs_width.clamp(140, 420),
        graph_width: columns.graph_width.clamp(64, 720),
        graph_columns: keep_graph_columns(&columns.graph_columns),
    };
    let json = serde_json::to_string_pretty(&columns).map_err(|e| e.to_string())?;
    fs::write(dir.join("layout.json"), json).map_err(|e| e.to_string())
}

fn default_merge_template() -> String {
    "Merge branch '{branch}' into {current}".into()
}
fn default_stash_template() -> String {
    "WIP on {current}: {summary}".into()
}
fn default_cherry_template() -> String {
    "Cherry-pick {sha} onto {current}".into()
}
fn default_rebase_template() -> String {
    "Rebase {current} onto {branch}".into()
}
fn default_reword_template() -> String {
    "{summary}".into()
}
fn default_pr_template() -> String {
    "[{ticket}] {summary}".into()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Templates {
    #[serde(default)]
    pub commit: String,
    #[serde(default = "default_merge_template")]
    pub merge: String,
    #[serde(default = "default_stash_template")]
    pub stash: String,
    #[serde(default = "default_cherry_template")]
    pub cherry: String,
    #[serde(default = "default_rebase_template")]
    pub rebase: String,
    #[serde(default = "default_reword_template")]
    pub reword: String,
    #[serde(default = "default_pr_template")]
    pub pr: String,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            commit: String::new(),
            merge: default_merge_template(),
            stash: default_stash_template(),
            cherry: default_cherry_template(),
            rebase: default_rebase_template(),
            reword: default_reword_template(),
            pr: default_pr_template(),
        }
    }
}

fn clean_template(value: &str) -> Result<String, String> {
    if value.contains('\0') {
        return Err("La plantilla no es válida.".into());
    }
    if value.len() > 4000 {
        return Err("La plantilla es demasiado larga.".into());
    }
    Ok(value.replace('\r', ""))
}

pub fn templates() -> Result<Templates, String> {
    let path = data_dir().join("templates.json");
    if !path.exists() {
        return Ok(Templates::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub fn save_templates(templates: &Templates) -> Result<Templates, String> {
    let dir = ensure()?;
    let templates = Templates {
        commit: clean_template(&templates.commit)?,
        merge: clean_template(&templates.merge)?,
        stash: clean_template(&templates.stash)?,
        cherry: clean_template(&templates.cherry)?,
        rebase: clean_template(&templates.rebase)?,
        reword: clean_template(&templates.reword)?,
        pr: clean_template(&templates.pr)?,
    };
    let json = serde_json::to_string_pretty(&templates).map_err(|e| e.to_string())?;
    fs::write(dir.join("templates.json"), json).map_err(|e| e.to_string())?;
    Ok(templates)
}

pub fn profile_for(path: &Path) -> Result<Option<Profile>, String> {
    let file = profiles()?;
    let path_text = path.to_string_lossy().to_string();
    let Some(link) = file.repos.iter().find(|item| item.path == path_text) else {
        return Ok(file.profiles.into_iter().find(|item| item.id == DEFAULT_ID));
    };
    if link.profile_id.is_empty() {
        return Ok(None);
    }
    Ok(file
        .profiles
        .into_iter()
        .find(|item| item.id == link.profile_id))
}

pub fn preferences() -> Result<Preferences, String> {
    let path = data_dir().join("preferences.json");
    if !path.exists() {
        return Ok(Preferences::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub fn save_preferences(prefs: &Preferences) -> Result<Preferences, String> {
    let dir = ensure()?;
    let json = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    fs::write(dir.join("preferences.json"), json).map_err(|e| e.to_string())?;
    Ok(prefs.clone())
}
