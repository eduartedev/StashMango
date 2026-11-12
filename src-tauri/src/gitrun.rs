use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Debug)]
pub struct Git {
    pub bin: PathBuf,
    pub bundled: bool,
}

#[derive(Debug)]
pub struct GitOut {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl GitOut {
    pub fn ok(&self) -> bool {
        self.code == 0
    }

    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    pub fn err_text(&self) -> String {
        let err = self.stderr.trim();
        if err.is_empty() {
            self.text().trim().to_string()
        } else {
            err.to_string()
        }
    }
}

pub fn resolve() -> Git {
    if let Some(bin) = on_path() {
        return Git { bin, bundled: false };
    }
    if let Some(bin) = bundled_bin() {
        return Git { bin, bundled: true };
    }
    Git {
        bin: PathBuf::from(if cfg!(windows) { "git.exe" } else { "git" }),
        bundled: false,
    }
}

pub fn on_path() -> Option<PathBuf> {
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn bundled_bin() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("STASHMANGO_GIT") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let name = if cfg!(windows) { "git.exe" } else { "git" };
    let candidates = [
        dir.join("git").join("cmd").join(name),
        dir.join("git").join(name),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

pub fn bundled_dir() -> Option<PathBuf> {
    bundled_bin().and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

pub fn run(
    git: &Path,
    cwd: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
    stdin_data: Option<&[u8]>,
) -> Result<GitOut, String> {
    let mut cmd = Command::new(git);
    cmd.args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .stdin(if stdin_data.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo arrancar git: {e}"))?;
    if let Some(data) = stdin_data {
        if let Some(mut sin) = child.stdin.take() {
            sin.write_all(data).map_err(|e| e.to_string())?;
        }
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("git no terminó: {e}"))?;
    Ok(GitOut {
        code: out.status.code().unwrap_or(1),
        stdout: out.stdout,
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    })
}

pub fn run_ok(
    git: &Path,
    cwd: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
    stdin_data: Option<&[u8]>,
) -> Result<GitOut, String> {
    let out = run(git, cwd, args, extra_env, stdin_data)?;
    if out.ok() {
        Ok(out)
    } else {
        Err(out.err_text())
    }
}
