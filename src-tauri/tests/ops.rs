use stashmango_lib::gitrun;
use stashmango_lib::graph::{self, CommitNode};
use stashmango_lib::ops;
use stashmango_lib::pathcfg;
use stashmango_lib::prs;
use stashmango_lib::secret;
use stashmango_lib::store::{self, Profile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

static HOME_LOCK: Mutex<()> = Mutex::new(());

fn git() -> PathBuf {
    gitrun::on_path().expect("git en PATH")
}

fn run(git: &Path, cwd: &Path, args: &[&str]) {
    let status = Command::new(git)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success(), "{args:?}");
}

fn repo() -> (PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let git = git();
    run(&git, dir.path(), &["init", "-b", "main"]);
    run(&git, dir.path(), &["config", "user.name", "Otro"]);
    run(&git, dir.path(), &["config", "user.email", "otro@example.com"]);
    run(&git, dir.path(), &["config", "core.autocrlf", "false"]);
    (git, dir)
}

fn commit_file(git: &Path, root: &Path, name: &str, body: &str, message: &str) {
    fs::write(root.join(name), body).unwrap();
    run(git, root, &["add", "--", name]);
    run(git, root, &["commit", "-m", message]);
}

#[test]
fn not_a_repo_does_not_write() {
    let git = git();
    let dir = tempfile::tempdir().unwrap();
    let err = ops::open_repo(&git, dir.path()).unwrap_err();
    assert!(err.contains("no es un repositorio"));
    assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
}

#[test]
fn linked_worktree_opens() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    run(&git, dir.path(), &["branch", "otra"]);
    let other = dir.path().join("otra-carpeta");
    ops::add_worktree(&git, dir.path(), &other, "otra").unwrap();
    let info = ops::open_repo(&git, &other).unwrap();
    assert_eq!(info.branch, "otra");
    let list = ops::worktrees(&git, dir.path()).unwrap();
    assert!(list.iter().any(|w| w.branch == "otra"));
    ops::remove_worktree(&git, dir.path(), &other, true).unwrap();
    let list = ops::worktrees(&git, dir.path()).unwrap();
    assert!(!list.iter().any(|w| w.branch == "otra"));
    let again = dir.path().join("segunda");
    let err = ops::add_worktree(&git, dir.path(), &again, "main").unwrap_err();
    assert!(!err.is_empty());
    assert!(dir.path().join("a.txt").exists());
}

#[test]
fn prefers_path_git() {
    assert!(gitrun::on_path().is_some());
}

#[test]
fn path_toggle_appends_once_and_removes_only_that_entry() {
    let current = r"C:\Git\cmd;C:\Windows";
    let dir = r"C:\StashMango\git\cmd";
    let once = pathcfg::insert_path(current, dir, ';');
    let twice = pathcfg::insert_path(&once, dir, ';');
    assert_eq!(once, twice);
    assert!(twice.ends_with(dir));
    assert!(twice.contains(r"C:\Git\cmd"));
    let back = pathcfg::remove_path(&twice, dir, ';');
    assert_eq!(back, current);
    assert!(!pathcfg::local_bin_on_path("/usr/bin", "/home/a/.local/bin"));
    assert!(pathcfg::local_bin_on_path("/home/a/.local/bin:/usr/bin", "/home/a/.local/bin"));
}

#[test]
fn status_splits_groups() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    fs::write(dir.path().join("a.txt"), "dos\n").unwrap();
    fs::write(dir.path().join("nuevo.txt"), "x\n").unwrap();
    run(&git, dir.path(), &["add", "a.txt"]);
    fs::write(dir.path().join("a.txt"), "tres\n").unwrap();
    let files = ops::status(&git, dir.path()).unwrap();
    let a = files.iter().find(|f| f.path == "a.txt").unwrap();
    assert!(a.staged && a.unstaged);
    let nuevo = files.iter().find(|f| f.path == "nuevo.txt").unwrap();
    assert!(nuevo.untracked);
}

#[test]
fn stage_and_unstage_file() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    fs::write(dir.path().join("a.txt"), "dos\n").unwrap();
    ops::stage_file(&git, dir.path(), "a.txt", true).unwrap();
    let files = ops::status(&git, dir.path()).unwrap();
    assert!(files.iter().any(|f| f.path == "a.txt" && f.staged));
    ops::stage_file(&git, dir.path(), "a.txt", false).unwrap();
    let files = ops::status(&git, dir.path()).unwrap();
    assert!(files.iter().any(|f| f.path == "a.txt" && f.unstaged && !f.staged));
}

#[test]
fn stage_one_line() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "a\nb\nc\nd\n", "init");
    fs::write(dir.path().join("a.txt"), "a\nB\nc\nD\n").unwrap();
    let diff = ops::diff_file(&git, dir.path(), "a.txt", false).unwrap();
    assert!(diff.hunks >= 2, "{}", diff.text);
    ops::apply_selection(&git, dir.path(), "a.txt", false, &[0], &[]).unwrap();
    let cached = ops::diff_file(&git, dir.path(), "a.txt", true).unwrap();
    assert!(cached.text.contains("+B"), "{}", cached.text);
    assert!(!cached.text.contains("+D"), "{}", cached.text);
}

#[test]
fn commit_with_profile_does_not_change_config() {
    let _lock = HOME_LOCK.lock().unwrap();
    let (git, dir) = repo();
    let home = tempfile::tempdir().unwrap();
    // El mutex de estas pruebas evita la carrera que hace inseguro set_var.
    unsafe { std::env::set_var("STASHMANGO_HOME", home.path()); }
    fs::write(dir.path().join("a.txt"), "uno\n").unwrap();
    ops::stage_file(&git, dir.path(), "a.txt", true).unwrap();
    ops::commit(&git, dir.path(), "hola", Some("Mango"), Some("mango@example.com")).unwrap();
    let author = gitrun::run_ok(&git, dir.path(), &["log", "-1", "--format=%an %ae"], &[], None).unwrap();
    assert_eq!(author.text().trim(), "Mango mango@example.com");
    assert_eq!(ops::local_config(&git, dir.path(), "user.name").unwrap(), "Otro");
    let err = store::validate_identity("Ana", "ana\n@example.com").unwrap_err();
    assert!(err.contains("correo"));
}

#[test]
fn branches_carry_the_remote_commit() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    let bare = tempfile::tempdir().unwrap();
    run(&git, bare.path(), &["init", "--bare"]);
    let remote = bare.path().to_string_lossy().to_string();
    run(&git, dir.path(), &["remote", "add", "origin", &remote]);
    run(&git, dir.path(), &["push", "-u", "origin", "HEAD"]);
    run(&git, dir.path(), &["remote", "set-head", "origin", "main"]);
    let refs = ops::branches(&git, dir.path()).unwrap();
    let main = refs.iter().find(|item| item.name == "main" && !item.remote).unwrap();
    let origin = refs.iter().find(|item| item.name == "origin/main" && item.remote).unwrap();
    assert_eq!(main.sha, origin.sha);
    assert!(refs.iter().any(|item| item.name == "origin/HEAD" && item.remote));
    run(&git, dir.path(), &["commit", "--allow-empty", "-m", "adelante"]);
    let later = ops::branches(&git, dir.path()).unwrap();
    let local = later.iter().find(|item| item.name == "main" && !item.remote).unwrap();
    let remote_main = later.iter().find(|item| item.name == "origin/main" && item.remote).unwrap();
    assert_ne!(local.sha, remote_main.sha);
}

#[test]
fn invalid_branch_is_rejected() {
    let (git, dir) = repo();
    assert!(ops::create_branch(&git, dir.path(), "HEAD").is_err());
    assert!(ops::create_branch(&git, dir.path(), "rama mala").is_err());
}

#[test]
fn stash_roundtrip() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    fs::write(dir.path().join("a.txt"), "dos\n").unwrap();
    ops::stash_push(&git, dir.path(), "guarda").unwrap();
    let text = fs::read_to_string(dir.path().join("a.txt")).unwrap();
    assert_eq!(text, "uno\n");
    ops::stash_pop(&git, dir.path()).unwrap();
    let text = fs::read_to_string(dir.path().join("a.txt")).unwrap();
    assert_eq!(text, "dos\n");
}

#[test]
fn merge_conflict_and_ours() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "base\n", "init");
    run(&git, dir.path(), &["branch", "lado"]);
    fs::write(dir.path().join("a.txt"), "nuestro\n").unwrap();
    run(&git, dir.path(), &["commit", "-am", "nuestro"]);
    run(&git, dir.path(), &["switch", "lado"]);
    fs::write(dir.path().join("a.txt"), "suyo\n").unwrap();
    run(&git, dir.path(), &["commit", "-am", "suyo"]);
    run(&git, dir.path(), &["switch", "main"]);
    let merged = ops::merge(&git, dir.path(), "lado", "").unwrap();
    assert!(!merged.ok());
    let unmerged = ops::unmerged(&git, dir.path()).unwrap();
    assert!(unmerged.contains("a.txt"));
    assert!(unmerged.lines().any(|l| l.starts_with('1') || l.contains(" 1")));
    ops::resolve_side(&git, dir.path(), "a.txt", true, true).unwrap();
    let unmerged = ops::unmerged(&git, dir.path()).unwrap();
    assert!(!unmerged.contains("a.txt"), "{unmerged}");
}

#[test]
fn rebase_and_cherry_pick() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    run(&git, dir.path(), &["branch", "feature"]);
    fs::write(dir.path().join("a.txt"), "main\n").unwrap();
    run(&git, dir.path(), &["commit", "-am", "en main"]);
    run(&git, dir.path(), &["switch", "feature"]);
    fs::write(dir.path().join("b.txt"), "feat\n").unwrap();
    run(&git, dir.path(), &["add", "b.txt"]);
    run(&git, dir.path(), &["commit", "-m", "feat"]);
    let sha = ops::head_sha(&git, dir.path()).unwrap();
    run(&git, dir.path(), &["switch", "main"]);
    ops::cherry_pick(&git, dir.path(), &sha, "").unwrap();
    assert!(dir.path().join("b.txt").exists());
    run(&git, dir.path(), &["switch", "feature"]);
    ops::rebase(&git, dir.path(), "main").unwrap();
}

#[test]
fn reset_hard_can_return() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "primero");
    let first = ops::head_sha(&git, dir.path()).unwrap();
    commit_file(&git, dir.path(), "a.txt", "dos\n", "segundo");
    let previous = ops::reset_hard(&git, dir.path(), &first, true).unwrap();
    assert_eq!(ops::head_sha(&git, dir.path()).unwrap(), first);
    ops::reset_hard(&git, dir.path(), &previous, true).unwrap();
    let text = fs::read_to_string(dir.path().join("a.txt")).unwrap();
    assert_eq!(text, "dos\n");
    assert!(ops::reset_hard(&git, dir.path(), &first, false).is_err());
}

#[test]
fn log_page_reads_history() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "primero");
    commit_file(&git, dir.path(), "a.txt", "dos\n", "segundo");
    let page = ops::log_page(&git, dir.path(), 0, Vec::new()).unwrap();
    assert!(page.commits.len() >= 2, "{:?}", page.commits);
    assert_eq!(page.commits[0].summary, "segundo");
    assert!(!page.commits[0].parents.is_empty());
}

#[test]
fn lanes_continue_across_pages() {
    let mut first = vec![node("c4", &["c2", "c3"]), node("c2", &["c1"])];
    let mut open = Vec::new();
    graph::place(&mut first, &mut open);
    let mut second = vec![node("c3", &["c1"]), node("c1", &[])];
    graph::place(&mut second, &mut open);
    assert_eq!(first[0].lane, 0);
    assert_eq!(first[0].parent_lanes, vec![0, 1]);
    assert_eq!(second[0].lane, 1);
    assert_eq!(second[1].lane, 0);
}

#[test]
fn big_diff_hides_body_and_rejects_escape() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    fs::write(dir.path().join("grande.txt"), vec![b'a'; 1_048_577]).unwrap();
    run(&git, dir.path(), &["add", "grande.txt"]);
    let diff = ops::diff_file(&git, dir.path(), "grande.txt", true).unwrap();
    assert!(diff.too_big);
    assert!(diff.text.is_empty());
    assert!(ops::open_external(dir.path(), "../fuera.txt").is_err());
}

#[test]
fn push_to_local_remote_without_token() {
    let _lock = HOME_LOCK.lock().unwrap();
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "uno\n", "init");
    let bare = tempfile::tempdir().unwrap();
    run(&git, bare.path(), &["init", "--bare"]);
    let remote = bare.path().to_string_lossy().to_string();
    run(&git, dir.path(), &["remote", "add", "origin", &remote]);
    run(&git, dir.path(), &["push", "-u", "origin", "HEAD"]);
    let args = ops::push_command_args(Path::new("C:/StashMango/stashmango.exe"));
    let secret = "ghp_supersecreto";
    assert!(args.iter().all(|arg| !arg.contains(secret)));
    let home = tempfile::tempdir().unwrap();
    // El mutex de estas pruebas evita la carrera que hace inseguro set_var.
    unsafe { std::env::set_var("STASHMANGO_HOME", home.path()); }
    store::remember(dir.path(), "main").unwrap();
    let json = fs::read_to_string(home.path().join("recents.json")).unwrap();
    assert!(!json.contains(secret));
    assert!(secret::validate_token(secret).is_ok());
    assert!(secret::validate_token("con espacio").is_err());
}

#[test]
fn remote_parser_knows_the_three_forges() {
    let gh = prs::parse_remote("git@github.com:eduartedev/StashMango.git").unwrap();
    assert_eq!(gh.forge, "github");
    assert_eq!(gh.owner, "eduartedev");
    let gl = prs::parse_remote("https://gitlab.com/grupo/equipo/app.git").unwrap();
    assert_eq!(gl.forge, "gitlab");
    assert_eq!(gl.name, "app");
    let bb = prs::parse_remote("https://bitbucket.org/ws/repo").unwrap();
    assert_eq!(bb.forge, "bitbucket");
}

#[test]
fn default_profile_is_present_and_tokens_stay_out_of_the_file() {
    let _lock = HOME_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    unsafe { std::env::set_var("STASHMANGO_HOME", home.path()); }
    let file = store::profiles().unwrap();
    assert_eq!(file.profiles[0].id, "default");
    assert_eq!(file.profiles[0].label, "default");
    assert!(store::delete_profile("default").is_err());
    store::save_profile(Profile {
        id: "default".into(),
        label: "default".into(),
        name: "Mango".into(),
        email: "mango@example.com".into(),
    })
    .unwrap();
    let again = store::profile_for(Path::new("C:/repos/sin-enlace")).unwrap();
    assert_eq!(again.unwrap().email, "mango@example.com");
    store::set_repo_profile(Path::new("C:/repos/sin-enlace"), None).unwrap();
    assert!(store::profile_for(Path::new("C:/repos/sin-enlace")).unwrap().is_none());
    let account = secret::token_account("default", "github.com").unwrap();
    let other = secret::token_account("otro", "github.com").unwrap();
    assert_eq!(account, "default@github.com");
    assert_ne!(account, other);
    let json = fs::read_to_string(home.path().join("profiles.json")).unwrap();
    assert!(!json.contains("ghp_"));
    assert!(!json.contains("token"));
}

#[test]
fn layout_roundtrip_keeps_closed_columns() {
    let _lock = HOME_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    unsafe { std::env::set_var("STASHMANGO_HOME", home.path()); }
    let fresh = store::layout().unwrap();
    assert!(fresh.side && fresh.detail);
    store::save_layout(&store::Columns {
        side: false,
        detail: true,
        side_width: 9000,
        detail_width: 400,
        refs_width: 10,
        graph_width: 4000,
        graph_columns: vec!["hash".into(), "nope".into(), "refs".into()],
    })
    .unwrap();
    let again = store::layout().unwrap();
    assert!(!again.side && again.detail);
    assert_eq!(again.side_width, 480);
    assert_eq!(again.detail_width, 400);
    assert_eq!(again.refs_width, 140);
    assert_eq!(again.graph_width, 720);
    assert_eq!(again.graph_columns, vec!["refs".to_string(), "hash".to_string()]);
    fs::write(home.path().join("layout.json"), r#"{"side":false,"detail":true}"#).unwrap();
    let partial = store::layout().unwrap();
    assert!(!partial.side && partial.detail);
    assert_eq!(partial.side_width, 268);
    assert_eq!(
        partial.graph_columns,
        vec!["refs".to_string(), "graph".to_string(), "message".to_string(), "date".to_string()]
    );
    let json = fs::read_to_string(home.path().join("layout.json")).unwrap();
    assert!(json.contains("\"side\":false"));
    assert!(!json.contains("token"));
    fs::write(home.path().join("layout.json"), "{").unwrap();
    let broken = store::layout().unwrap();
    assert!(broken.side && broken.detail);
}

#[test]
fn profile_roundtrip_has_no_token_field() {
    let _lock = HOME_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    // El mutex de estas pruebas evita la carrera que hace inseguro set_var.
    unsafe { std::env::set_var("STASHMANGO_HOME", home.path()); }
    store::save_profile(Profile {
        id: "1".into(),
        label: "Personal".into(),
        name: "Mango".into(),
        email: "mango@example.com".into(),
    })
    .unwrap();
    let json = fs::read_to_string(home.path().join("profiles.json")).unwrap();
    assert!(!json.contains("token"));
    assert!(json.contains("mango@example.com"));
}

#[test]
fn templates_roundtrip_keeps_merge_text() {
    let _lock = HOME_LOCK.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    unsafe { std::env::set_var("STASHMANGO_HOME", home.path()); }
    let fresh = store::templates().unwrap();
    assert_eq!(fresh.merge, "Merge branch '{branch}' into {current}");
    let mut saved = store::Templates::default();
    saved.commit = "WIP: ".into();
    store::save_templates(&saved).unwrap();
    let loaded = store::templates().unwrap();
    assert_eq!(loaded.commit, "WIP: ");
    fs::write(home.path().join("templates.json"), "{").unwrap();
    let broken = store::templates().unwrap();
    assert_eq!(broken.merge, "Merge branch '{branch}' into {current}");
}

#[test]
fn merge_uses_custom_message() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "base\n", "init");
    run(&git, dir.path(), &["branch", "lado"]);
    run(&git, dir.path(), &["switch", "lado"]);
    fs::write(dir.path().join("b.txt"), "extra\n").unwrap();
    run(&git, dir.path(), &["add", "b.txt"]);
    run(&git, dir.path(), &["commit", "-m", "lado"]);
    run(&git, dir.path(), &["switch", "main"]);
    ops::merge(&git, dir.path(), "lado", "Merge branch 'lado' into main").unwrap();
    let log = gitrun::run_ok(&git, dir.path(), &["log", "-1", "--format=%s"], &[], None).unwrap();
    assert_eq!(log.text().trim(), "Merge branch 'lado' into main");
}

#[test]
fn preview_file_covers_untracked_image_and_commit() {
    let (git, dir) = repo();
    commit_file(&git, dir.path(), "a.txt", "hola\n", "init");
    let sha = gitrun::run_ok(&git, dir.path(), &["rev-parse", "HEAD"], &[], None)
        .unwrap()
        .text()
        .trim()
        .to_string();
    fs::write(dir.path().join("nuevo.txt"), "sin seguimiento\n").unwrap();
    let untracked = ops::preview_file(&git, dir.path(), "nuevo.txt", false, "").unwrap();
    assert_eq!(untracked.kind, "text");
    assert!(untracked.text.contains("sin seguimiento"), "{}", untracked.text);

    fs::write(dir.path().join("pic.png"), TINY_PNG).unwrap();
    let image = ops::preview_file(&git, dir.path(), "pic.png", false, "").unwrap();
    assert_eq!(image.kind, "image");
    assert_eq!(image.media, "image/png");
    assert!(!image.data.is_empty());

    let commit = ops::preview_file(&git, dir.path(), "a.txt", false, &sha).unwrap();
    assert_eq!(commit.kind, "diff");
    assert_eq!(commit.sha, sha);
    assert!(commit.text.contains("hola") || commit.text.contains("a.txt"), "{}", commit.text);
}

const TINY_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
    0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
    0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
    0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

fn node(id: &str, parents: &[&str]) -> CommitNode {
    CommitNode {
        id: id.into(),
        parents: parents.iter().map(|p| (*p).to_string()).collect(),
        summary: id.into(),
        author: String::new(),
        email: String::new(),
        time: 0,
        refs: String::new(),
        lane: 0,
        parent_lanes: Vec::new(),
    }
}
