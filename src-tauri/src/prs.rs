use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
    pub head: String,
    pub base: String,
    pub forge: String,
}

#[derive(Clone, Debug)]
pub struct RemoteRepo {
    pub forge: String,
    pub api: String,
    pub owner: String,
    pub name: String,
    pub host: String,
}

pub fn parse_remote(url: &str) -> Option<RemoteRepo> {
    let url = url.trim().trim_end_matches(".git");
    let (host, path) = if let Some(rest) = url.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        (host.to_string(), path.to_string())
    } else if let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))
    {
        let (host, path) = rest.split_once('/')?;
        let host = host.split('@').last().unwrap_or(host);
        (host.to_string(), path.to_string())
    } else {
        return None;
    };
    let mut parts = path.split('/').filter(|p| !p.is_empty());
    let owner = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    if parts.next().is_some() && host != "gitlab.com" {
        // GitLab permite grupos anidados: el owner es todo menos el último.
    }
    let (owner, name) = if host == "gitlab.com" {
        let all: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        if all.len() < 2 {
            return None;
        }
        let name = all.last()?.to_string();
        let owner = all[..all.len() - 1].join("/");
        (owner, name)
    } else {
        (owner, name)
    };
    let forge = if host == "github.com" {
        "github"
    } else if host == "gitlab.com" {
        "gitlab"
    } else if host == "bitbucket.org" {
        "bitbucket"
    } else {
        return None;
    };
    Some(RemoteRepo {
        forge: forge.into(),
        api: String::new(),
        owner,
        name,
        host,
    })
}

pub fn list_prs(remote: &str, token: &str) -> Result<Vec<PullRequest>, String> {
    let repo = parse_remote(remote).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    match repo.forge.as_str() {
        "github" => github_list(&repo, token),
        "gitlab" => gitlab_list(&repo, token),
        "bitbucket" => bitbucket_list(&repo, token),
        _ => Err("Forja no reconocida.".into()),
    }
}

pub fn create_pr(
    remote: &str,
    token: &str,
    title: &str,
    head: &str,
    base: &str,
) -> Result<PullRequest, String> {
    let repo = parse_remote(remote).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    if title.trim().is_empty() {
        return Err("El pull request necesita un título.".into());
    }
    match repo.forge.as_str() {
        "github" => github_create(&repo, token, title, head, base),
        "gitlab" => gitlab_create(&repo, token, title, head, base),
        "bitbucket" => bitbucket_create(&repo, token, title, head, base),
        _ => Err("Forja no reconocida.".into()),
    }
}

pub fn merge_pr(remote: &str, token: &str, number: u64) -> Result<(), String> {
    let repo = parse_remote(remote).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    match repo.forge.as_str() {
        "github" => github_merge(&repo, token, number),
        "gitlab" => gitlab_merge(&repo, token, number),
        "bitbucket" => bitbucket_merge(&repo, token, number),
        _ => Err("Forja no reconocida.".into()),
    }
}

pub fn pr_diff(remote: &str, token: &str, number: u64) -> Result<String, String> {
    let repo = parse_remote(remote).ok_or("El remoto no es GitHub, GitLab ni Bitbucket.")?;
    let url = match repo.forge.as_str() {
        "github" => format!(
            "https://api.github.com/repos/{}/{}/pulls/{number}",
            repo.owner, repo.name
        ),
        "gitlab" => {
            let project = urlencoding(&format!("{}/{}", repo.owner, repo.name));
            format!("https://gitlab.com/api/v4/projects/{project}/merge_requests/{number}/changes")
        }
        "bitbucket" => format!(
            "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests/{number}/diff",
            repo.owner, repo.name
        ),
        _ => return Err("Forja no reconocida.".into()),
    };
    let mut req = ureq::get(&url).set("User-Agent", "StashMango");
    req = match repo.forge.as_str() {
        "github" => req
            .set("Authorization", &format!("Bearer {token}"))
            .set("Accept", "application/vnd.github.diff"),
        "gitlab" => req.set("PRIVATE-TOKEN", token),
        _ => req.set("Authorization", &format!("Bearer {token}")),
    };
    let text = req.call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    if text.len() > 1_048_576 {
        return Err("El diff del pull request pasa de 1 MB.".into());
    }
    Ok(text)
}

fn github_list(repo: &RemoteRepo, token: &str) -> Result<Vec<PullRequest>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls?state=open",
        repo.owner, repo.name
    );
    let json: serde_json::Value = get_json(&url, &[("Authorization", &format!("Bearer {token}")), ("Accept", "application/vnd.github+json")])?;
    let mut out = Vec::new();
    for item in json.as_array().ok_or("Respuesta inesperada de GitHub.")? {
        out.push(PullRequest {
            id: item["node_id"].as_str().unwrap_or("").into(),
            number: item["number"].as_u64().unwrap_or(0),
            title: item["title"].as_str().unwrap_or("").into(),
            author: item["user"]["login"].as_str().unwrap_or("").into(),
            url: item["html_url"].as_str().unwrap_or("").into(),
            head: item["head"]["ref"].as_str().unwrap_or("").into(),
            base: item["base"]["ref"].as_str().unwrap_or("").into(),
            forge: "github".into(),
        });
    }
    Ok(out)
}

fn github_create(repo: &RemoteRepo, token: &str, title: &str, head: &str, base: &str) -> Result<PullRequest, String> {
    let url = format!("https://api.github.com/repos/{}/{}/pulls", repo.owner, repo.name);
    let body = serde_json::json!({ "title": title, "head": head, "base": base });
    let item = post_json(&url, &[("Authorization", &format!("Bearer {token}")), ("Accept", "application/vnd.github+json")], body)?;
    Ok(PullRequest {
        id: item["node_id"].as_str().unwrap_or("").into(),
        number: item["number"].as_u64().unwrap_or(0),
        title: item["title"].as_str().unwrap_or("").into(),
        author: item["user"]["login"].as_str().unwrap_or("").into(),
        url: item["html_url"].as_str().unwrap_or("").into(),
        head: head.into(),
        base: base.into(),
        forge: "github".into(),
    })
}

fn github_merge(repo: &RemoteRepo, token: &str, number: u64) -> Result<(), String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{number}/merge",
        repo.owner, repo.name
    );
    put_json(&url, &[("Authorization", &format!("Bearer {token}")), ("Accept", "application/vnd.github+json")], serde_json::json!({}))?;
    Ok(())
}

fn gitlab_list(repo: &RemoteRepo, token: &str) -> Result<Vec<PullRequest>, String> {
    let project = urlencoding(&format!("{}/{}", repo.owner, repo.name));
    let url = format!("https://gitlab.com/api/v4/projects/{project}/merge_requests?state=opened");
    let json: serde_json::Value = get_json(&url, &[("PRIVATE-TOKEN", token)])?;
    let mut out = Vec::new();
    for item in json.as_array().ok_or("Respuesta inesperada de GitLab.")? {
        out.push(PullRequest {
            id: item["iid"].to_string(),
            number: item["iid"].as_u64().unwrap_or(0),
            title: item["title"].as_str().unwrap_or("").into(),
            author: item["author"]["username"].as_str().unwrap_or("").into(),
            url: item["web_url"].as_str().unwrap_or("").into(),
            head: item["source_branch"].as_str().unwrap_or("").into(),
            base: item["target_branch"].as_str().unwrap_or("").into(),
            forge: "gitlab".into(),
        });
    }
    Ok(out)
}

fn gitlab_create(repo: &RemoteRepo, token: &str, title: &str, head: &str, base: &str) -> Result<PullRequest, String> {
    let project = urlencoding(&format!("{}/{}", repo.owner, repo.name));
    let url = format!("https://gitlab.com/api/v4/projects/{project}/merge_requests");
    let item = post_json(&url, &[("PRIVATE-TOKEN", token)], serde_json::json!({
        "title": title,
        "source_branch": head,
        "target_branch": base
    }))?;
    Ok(PullRequest {
        id: item["iid"].to_string(),
        number: item["iid"].as_u64().unwrap_or(0),
        title: title.into(),
        author: String::new(),
        url: item["web_url"].as_str().unwrap_or("").into(),
        head: head.into(),
        base: base.into(),
        forge: "gitlab".into(),
    })
}

fn gitlab_merge(repo: &RemoteRepo, token: &str, number: u64) -> Result<(), String> {
    let project = urlencoding(&format!("{}/{}", repo.owner, repo.name));
    let url = format!("https://gitlab.com/api/v4/projects/{project}/merge_requests/{number}/merge");
    put_json(&url, &[("PRIVATE-TOKEN", token)], serde_json::json!({}))?;
    Ok(())
}

fn bitbucket_list(repo: &RemoteRepo, token: &str) -> Result<Vec<PullRequest>, String> {
    let url = format!(
        "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests",
        repo.owner, repo.name
    );
    let json: serde_json::Value = get_json(&url, &[("Authorization", &format!("Bearer {token}"))])?;
    let mut out = Vec::new();
    for item in json["values"].as_array().ok_or("Respuesta inesperada de Bitbucket.")? {
        out.push(PullRequest {
            id: item["id"].to_string(),
            number: item["id"].as_u64().unwrap_or(0),
            title: item["title"].as_str().unwrap_or("").into(),
            author: item["author"]["display_name"].as_str().unwrap_or("").into(),
            url: item["links"]["html"]["href"].as_str().unwrap_or("").into(),
            head: item["source"]["branch"]["name"].as_str().unwrap_or("").into(),
            base: item["destination"]["branch"]["name"].as_str().unwrap_or("").into(),
            forge: "bitbucket".into(),
        });
    }
    Ok(out)
}

fn bitbucket_create(repo: &RemoteRepo, token: &str, title: &str, head: &str, base: &str) -> Result<PullRequest, String> {
    let url = format!(
        "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests",
        repo.owner, repo.name
    );
    let item = post_json(&url, &[("Authorization", &format!("Bearer {token}"))], serde_json::json!({
        "title": title,
        "source": { "branch": { "name": head } },
        "destination": { "branch": { "name": base } }
    }))?;
    Ok(PullRequest {
        id: item["id"].to_string(),
        number: item["id"].as_u64().unwrap_or(0),
        title: title.into(),
        author: String::new(),
        url: item["links"]["html"]["href"].as_str().unwrap_or("").into(),
        head: head.into(),
        base: base.into(),
        forge: "bitbucket".into(),
    })
}

fn bitbucket_merge(repo: &RemoteRepo, token: &str, number: u64) -> Result<(), String> {
    let url = format!(
        "https://api.bitbucket.org/2.0/repositories/{}/{}/pullrequests/{number}/merge",
        repo.owner, repo.name
    );
    post_json(&url, &[("Authorization", &format!("Bearer {token}"))], serde_json::json!({}))?;
    Ok(())
}

fn get_json(url: &str, headers: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    let mut req = ureq::get(url).set("User-Agent", "StashMango");
    for (key, value) in headers {
        req = req.set(key, value);
    }
    req.call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())
}

fn post_json(url: &str, headers: &[(&str, &str)], body: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut req = ureq::post(url).set("User-Agent", "StashMango");
    for (key, value) in headers {
        req = req.set(key, value);
    }
    req.send_json(body)
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())
}

fn put_json(url: &str, headers: &[(&str, &str)], body: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut req = ureq::put(url).set("User-Agent", "StashMango");
    for (key, value) in headers {
        req = req.set(key, value);
    }
    req.send_json(body)
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())
}

fn urlencoding(value: &str) -> String {
    value.replace('/', "%2F")
}
