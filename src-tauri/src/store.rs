use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recent {
    pub path: String,
    pub name: String,
    pub branch: String,
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
