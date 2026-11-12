pub fn insert_path(current: &str, dir: &str, sep: char) -> String {
    let dir = trim_dir(dir);
    let mut parts: Vec<&str> = current
        .split(sep)
        .filter(|part| !part.is_empty() && !same(part, dir))
        .collect();
    parts.push(dir);
    parts.join(&sep.to_string())
}

pub fn remove_path(current: &str, dir: &str, sep: char) -> String {
    let dir = trim_dir(dir);
    current
        .split(sep)
        .filter(|part| !part.is_empty() && !same(part, dir))
        .collect::<Vec<_>>()
        .join(&sep.to_string())
}

pub fn local_bin_on_path(path: &str, local_bin: &str) -> bool {
    path.split(':')
        .any(|part| same(part, local_bin))
}

fn trim_dir(dir: &str) -> &str {
    dir.trim_end_matches(['\\', '/'])
}

fn same(left: &str, right: &str) -> bool {
    trim_dir(left).eq_ignore_ascii_case(trim_dir(right))
}

#[cfg(windows)]
pub fn set_user_path(dir: &std::path::Path, enabled: bool) -> Result<(), String> {
    use winreg::enums::*;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .map_err(|e| format!("No se pudo abrir el PATH del usuario: {e}"))?;
    let current: String = env.get_value("Path").unwrap_or_default();
    let dir = dir.to_string_lossy();
    let next = if enabled {
        insert_path(&current, &dir, ';')
    } else {
        remove_path(&current, &dir, ';')
    };
    env.set_value("Path", &next)
        .map_err(|e| format!("No se pudo guardar el PATH: {e}"))?;
    Ok(())
}
