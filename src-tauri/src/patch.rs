#[derive(Clone, Debug)]
pub struct Hunk {
    pub header: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct FileDiff {
    pub preamble: String,
    pub hunks: Vec<Hunk>,
    pub binary: bool,
}

pub fn parse(text: &str) -> FileDiff {
    if text.contains("Binary files") || text.contains("GIT binary patch") {
        return FileDiff {
            preamble: text.to_string(),
            hunks: Vec::new(),
            binary: true,
        };
    }
    let mut preamble = String::new();
    let mut hunks = Vec::new();
    let mut current: Option<Hunk> = None;
    for line in text.split_inclusive('\n') {
        if line.starts_with("@@") {
            if let Some(hunk) = current.take() {
                hunks.push(hunk);
            }
            current = Some(Hunk {
                header: line.trim_end_matches(['\n', '\r']).to_string(),
                lines: Vec::new(),
            });
        } else if let Some(hunk) = current.as_mut() {
            hunk.lines.push(line.trim_end_matches(['\n', '\r']).to_string());
        } else {
            preamble.push_str(line);
        }
    }
    if let Some(hunk) = current {
        hunks.push(hunk);
    }
    FileDiff {
        preamble,
        hunks,
        binary: false,
    }
}

pub fn render(diff: &FileDiff, hunk_indexes: &[usize]) -> String {
    let mut out = diff.preamble.clone();
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    for index in hunk_indexes {
        let Some(hunk) = diff.hunks.get(*index) else {
            continue;
        };
        out.push_str(&hunk.header);
        out.push('\n');
        for line in &hunk.lines {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Parte un hunk -U0 en hunks de una sola línea cambiada.
pub fn split_lines(hunk: &Hunk) -> Vec<Hunk> {
    let (mut old, mut new) = header_starts(&hunk.header);
    let mut out = Vec::new();
    for line in &hunk.lines {
        if line.starts_with('\\') {
            continue;
        }
        let prefix = line.chars().next().unwrap_or(' ');
        let body = line.get(1..).unwrap_or("");
        match prefix {
            '-' => {
                out.push(Hunk {
                    header: format!("@@ -{old},1 +{old},0 @@"),
                    lines: vec![format!("-{body}")],
                });
                old += 1;
            }
            '+' => {
                out.push(Hunk {
                    header: format!("@@ -{old},0 +{new} @@"),
                    lines: vec![format!("+{body}")],
                });
                new += 1;
            }
            _ => {
                old += 1;
                new += 1;
            }
        }
    }
    out
}

fn header_starts(header: &str) -> (u32, u32) {
    let mut old = 1u32;
    let mut new = 1u32;
    if let Some(rest) = header.strip_prefix("@@ ") {
        let mut parts = rest.split_whitespace();
        if let Some(old_part) = parts.next() {
            old = number_after(old_part);
        }
        if let Some(new_part) = parts.next() {
            new = number_after(new_part);
        }
    }
    (old, new)
}

fn number_after(token: &str) -> u32 {
    token
        .trim_start_matches(['-', '+'])
        .split(',')
        .next()
        .unwrap_or("1")
        .parse()
        .unwrap_or(1)
}
