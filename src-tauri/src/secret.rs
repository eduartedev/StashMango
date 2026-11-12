const SERVICE: &str = "StashMango";

pub fn validate_token(token: &str) -> Result<(), String> {
    if token.is_empty()
        || token.len() > 512
        || token.chars().any(|c| c.is_control() || c.is_whitespace())
    {
        return Err("El token no es válido.".into());
    }
    Ok(())
}

pub fn validate_host(host: &str) -> Result<(), String> {
    if host.is_empty()
        || host.len() > 253
        || host.contains(['/', '\\', ' ', '\n', '\r', '@'])
    {
        return Err("El host no es válido.".into());
    }
    Ok(())
}

pub fn token_account(profile_id: &str, host: &str) -> Result<String, String> {
    validate_host(host)?;
    if profile_id.is_empty()
        || profile_id.len() > 80
        || !profile_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("El perfil no es válido.".into());
    }
    Ok(format!("{profile_id}@{host}"))
}

pub fn save_token(profile_id: &str, host: &str, token: &str) -> Result<(), String> {
    let account = token_account(profile_id, host)?;
    validate_token(token)?;
    let entry = keyring::Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
    entry.set_password(token).map_err(|e| e.to_string())
}

pub fn read_token(profile_id: &str, host: &str) -> Result<Option<String>, String> {
    let account = token_account(profile_id, host)?;
    let entry = keyring::Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(token) => Ok(Some(token)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(err) => Err(err.to_string()),
    }
}

pub fn has_token(profile_id: &str, host: &str) -> bool {
    read_token(profile_id, host).ok().flatten().is_some()
}

pub fn clear_token(profile_id: &str, host: &str) -> Result<(), String> {
    let account = token_account(profile_id, host)?;
    let entry = keyring::Entry::new(SERVICE, &account).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

/// Respuesta del protocolo `git credential` para `get`. El token no se registra.
pub fn credential_get(input: &str) -> Result<String, String> {
    let mut host = None;
    let mut protocol = None;
    let mut username = None;
    for line in input.lines() {
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "host" => host = Some(value.to_string()),
                "protocol" => protocol = Some(value.to_string()),
                "username" => username = Some(value.to_string()),
                _ => {}
            }
        }
    }
    let Some(host) = host else {
        return Ok(String::new());
    };
    let profile = std::env::var("STASHMANGO_PROFILE").unwrap_or_else(|_| "default".into());
    let Some(token) = read_token(&profile, &host)? else {
        return Ok(String::new());
    };
    let user = username.unwrap_or_else(|| "git".into());
    let protocol = protocol.unwrap_or_else(|| "https".into());
    Ok(format!(
        "protocol={protocol}\nhost={host}\nusername={user}\npassword={token}\n"
    ))
}
