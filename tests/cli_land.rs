mod common;

use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use jj_lib::ref_name::WorkspaceName;
use jj_lib::repo::Repo;
use mockito::{Matcher, Server};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn plue_cmd() -> Command {
    cargo_bin_cmd!("plue")
}

fn write_config(config_home: &Path, api_url: &str) {
    let candidates = [
        config_home.join("plue"),
        config_home.join(".config").join("plue"),
        config_home
            .join("Library")
            .join("Application Support")
            .join("plue"),
    ];
    let config = format!("api_url: {api_url}\ntoken: plue_testtoken\n");
    for config_dir in candidates {
        fs::create_dir_all(&config_dir).expect("create config dir");
        fs::write(config_dir.join("config.yml"), &config).expect("write config");
    }
}

fn run_plue(args: &[&str], cwd: &Path, config_home: &Path) -> std::process::Output {
    plue_cmd()
        .args(args)
        .current_dir(cwd)
        .env("XDG_CONFIG_HOME", config_home)
        .env("HOME", config_home)
        .env("PLUE_TOKEN", "plue_testtoken")
        .output()
        .expect("run plue")
}

fn setup_temp_workspace() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = tmp.path().join("cfg");
    fs::create_dir_all(&cfg_home).expect("create cfg dir");
    (tmp, cfg_home)
}

#[test]
fn land_create_posts_request_and_prints_number_and_url() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/repos/alice/demo/landings")
        .match_header("authorization", "token plue_testtoken")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "title": "My change",
            "target_bookmark": "main",
            "change_ids": ["kseed001"]
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":42,"title":"My change","body":"","state":"open","author":{"id":1,"login":"alice"},"change_ids":["kseed001"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "land",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "My change",
            "--target",
            "main",
            "--change-id",
            "kseed001",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("42"), "stdout: {stdout}");
    assert!(
        stdout.contains("/alice/demo/landings/42"),
        "stdout: {stdout}"
    );

    create_mock.assert();
}

#[test]
fn land_create_auto_detects_current_change_id() {
    let mut server = Server::new();
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Auto detect", &[("a.txt", "a\n")]);
    let wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .expect("wc id");
    let commit = repo.store().get_commit(wc_id).expect("wc commit");
    let wc_change_id = commit.change_id().reverse_hex();

    let cfg_home = tempfile::TempDir::new().expect("cfg");
    write_config(cfg_home.path(), &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/repos/alice/demo/landings")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "change_ids": [wc_change_id],
            "target_bookmark": "main"
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":7,"title":"Auto detect","body":"","state":"open","author":{"id":1,"login":"alice"},"change_ids":["kseed001"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "land",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "Auto detect",
            "--target",
            "main",
        ],
        tmp.path(),
        cfg_home.path(),
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    create_mock.assert();
}

#[test]
fn land_create_stack_sends_multiple_change_ids() {
    let mut server = Server::new();
    let (tmp, _ws, repo) = common::init_test_repo();
    let repo = common::create_commit_with_files(&repo, &[], "Base", &[("a.txt", "a\n")]);
    let base_wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .expect("base wc")
        .clone();
    let repo = common::create_commit_with_files(&repo, &[&base_wc_id], "Top", &[("b.txt", "b\n")]);
    let top_wc_id = repo
        .view()
        .get_wc_commit_id(WorkspaceName::DEFAULT)
        .expect("top wc")
        .clone();
    let repo = common::create_commit_with_files(&repo, &[], "Unrelated", &[("z.txt", "z\n")]);
    let top_commit = repo.store().get_commit(&top_wc_id).expect("top commit");
    let base_commit = repo.store().get_commit(&base_wc_id).expect("base commit");
    let top_change_id = top_commit.change_id().reverse_hex();
    let base_change_id = base_commit.change_id().reverse_hex();

    let mut tx = repo.start_transaction();
    tx.repo_mut()
        .set_wc_commit(WorkspaceName::DEFAULT.to_owned(), top_wc_id)
        .expect("restore working copy to top");
    let _repo = tx.commit("restore wc to top").expect("commit tx");

    let cfg_home = tempfile::TempDir::new().expect("cfg");
    write_config(cfg_home.path(), &format!("{}/api", server.url()));

    let create_mock = server
        .mock("POST", "/api/repos/alice/demo/landings")
        .match_body(Matcher::AllOf(vec![
            Matcher::PartialJson(serde_json::json!({
                "title": "Stack",
                "target_bookmark": "main",
            })),
            Matcher::Regex(format!(
                r#""change_ids":\["{}","{}"\]"#,
                top_change_id, base_change_id
            )),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":8,"title":"Stack","body":"","state":"open","author":{"id":1,"login":"alice"},"change_ids":["k1","k2"],"target_bookmark":"main","conflict_status":"clean","stack_size":2,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "land",
            "create",
            "-R",
            "alice/demo",
            "--title",
            "Stack",
            "--target",
            "main",
            "--stack",
        ],
        tmp.path(),
        cfg_home.path(),
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    create_mock.assert();
}

#[test]
fn land_list_defaults_to_open_and_renders_table() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/landings")
        .match_query(Matcher::AllOf(vec![
            Matcher::UrlEncoded("page".into(), "1".into()),
            Matcher::UrlEncoded("per_page".into(), "30".into()),
            Matcher::UrlEncoded("state".into(), "open".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"number":5,"title":"Demo","body":"","state":"open","author":{"id":1,"login":"alice"},"change_ids":["k1"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(&["land", "list", "-R", "alice/demo"], tmp.path(), &cfg_home);

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Demo"), "stdout: {stdout}");
    assert!(stdout.to_lowercase().contains("number"), "stdout: {stdout}");
    assert!(
        stdout.to_lowercase().contains("change_ids"),
        "stdout: {stdout}"
    );
    assert!(stdout.contains("k1"), "stdout: {stdout}");

    list_mock.assert();
}

#[test]
fn land_list_json_field_filtering_keeps_requested_fields() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/landings")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"number":5,"title":"Demo","body":"ignore","state":"open","author":{"id":1,"login":"alice"},"change_ids":["k1"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &[
            "land",
            "list",
            "-R",
            "alice/demo",
            "--json",
            "number,title,state",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {e}\n{stdout}"));
    assert_eq!(value[0]["number"], 5);
    assert_eq!(value[0]["title"], "Demo");
    assert_eq!(value[0]["state"], "open");
    assert!(value[0].get("body").is_none());

    list_mock.assert();
}

#[test]
fn land_list_toon_output() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/landings")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"number":5,"title":"Demo item","body":"ignore","state":"open","author":{"id":1,"login":"alice"},"change_ids":["k1"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();

    let out = run_plue(
        &["land", "list", "-R", "alice/demo", "--toon"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("number:5"), "stdout: {stdout}");
    assert!(stdout.contains("title:\"Demo item\""), "stdout: {stdout}");
    assert!(stdout.contains("author.login:alice"), "stdout: {stdout}");
    assert!(!stdout.contains('{'), "stdout: {stdout}");

    list_mock.assert();
}

#[test]
fn land_view_fetches_details_changes_reviews_and_conflicts() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _landing = server
        .mock("GET", "/api/repos/alice/demo/landings/42")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":42,"title":"Demo","body":"body","state":"open","author":{"id":1,"login":"alice"},"change_ids":["k1"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();
    let _changes = server
        .mock("GET", "/api/repos/alice/demo/landings/42/changes")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"id":1,"landing_request_id":42,"change_id":"k1","position_in_stack":1,"created_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();
    let _reviews = server
        .mock("GET", "/api/repos/alice/demo/landings/42/reviews")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"id":1,"landing_request_id":42,"reviewer_id":2,"type":"comment","body":"LGTM","state":"submitted","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}]"#,
        )
        .create();
    let _conflicts = server
        .mock("GET", "/api/repos/alice/demo/landings/42/conflicts")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"conflict_status":"clean","has_conflicts":false,"conflicts_by_change":{}}"#)
        .create();

    let out = run_plue(
        &["land", "view", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Demo"), "stdout: {stdout}");
    assert!(stdout.contains("k1"), "stdout: {stdout}");
    assert!(stdout.contains("LGTM"), "stdout: {stdout}");
}

#[test]
fn land_review_approve_posts_review_type() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let review_mock = server
        .mock("POST", "/api/repos/alice/demo/landings/42/reviews")
        .match_body(Matcher::PartialJson(serde_json::json!({
            "type": "approve",
            "body": "LGTM"
        })))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"id":1,"landing_request_id":42,"reviewer_id":2,"type":"approve","body":"LGTM","state":"submitted","created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &[
            "land",
            "review",
            "42",
            "-R",
            "alice/demo",
            "--approve",
            "--body",
            "LGTM",
        ],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    review_mock.assert();
}

#[test]
fn land_merge_calls_land_endpoint() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let merge_mock = server
        .mock("PUT", "/api/repos/alice/demo/landings/42/land")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":42,"title":"Demo","body":"","state":"merged","author":{"id":1,"login":"alice"},"change_ids":["k1"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();

    let out = run_plue(
        &["land", "merge", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.to_lowercase().contains("merged"), "stdout: {stdout}");

    merge_mock.assert();
}

#[test]
fn land_checks_fetches_statuses() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _landing = server
        .mock("GET", "/api/repos/alice/demo/landings/42")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":42,"title":"Demo","body":"","state":"open","author":{"id":1,"login":"alice"},"change_ids":["kseed001"],"target_bookmark":"main","conflict_status":"clean","stack_size":1,"created_at":"2026-02-19T00:00:00Z","updated_at":"2026-02-19T00:00:00Z"}"#,
        )
        .create();
    let statuses_mock = server
        .mock("GET", "/api/repos/alice/demo/commits/kseed001/statuses")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"[{"context":"ci/test","status":"success","description":"All checks passed","target_url":"https://ci.example/test"}]"#,
        )
        .create();

    let out = run_plue(
        &["land", "checks", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("ci/test"), "stdout: {stdout}");
    statuses_mock.assert();
}

#[test]
fn land_list_without_repo_uses_origin_remote_auto_detection() {
    let git = StdCommand::new("git")
        .arg("--version")
        .status()
        .expect("git");
    assert!(git.success(), "git is required for this test");

    let mut server = Server::new();
    let tmp = TempDir::new().expect("tempdir");
    let cfg_home = TempDir::new().expect("cfg");
    write_config(cfg_home.path(), &format!("{}/api", server.url()));

    let init = StdCommand::new("git")
        .args(["init", "-q"])
        .current_dir(tmp.path())
        .status()
        .expect("git init");
    assert!(init.success());
    let remote = StdCommand::new("git")
        .args(["remote", "add", "origin", "git@plue.dev:alice/demo.git"])
        .current_dir(tmp.path())
        .status()
        .expect("git remote add");
    assert!(remote.success());

    let list_mock = server
        .mock("GET", "/api/repos/alice/demo/landings")
        .match_query(Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let out = run_plue(&["land", "list"], tmp.path(), cfg_home.path());
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    list_mock.assert();
}

#[test]
fn land_api_errors_surface_message() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _list_mock = server
        .mock("GET", "/api/repos/alice/demo/landings")
        .match_query(Matcher::Any)
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"repository not found"}"#)
        .create();

    let out = run_plue(&["land", "list", "-R", "alice/demo"], tmp.path(), &cfg_home);
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("repository not found"), "stderr: {stderr}");
}

#[test]
fn land_merge_not_found_shows_friendly_message() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _merge_mock = server
        .mock("PUT", "/api/repos/alice/demo/landings/42/land")
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"landing request not found"}"#)
        .create();

    let out = run_plue(
        &["land", "merge", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("landing request #42 was not found"),
        "stderr: {stderr}"
    );
}

#[test]
fn land_merge_conflict_shows_actionable_message() {
    let mut server = Server::new();
    let (tmp, cfg_home) = setup_temp_workspace();
    write_config(&cfg_home, &format!("{}/api", server.url()));

    let _merge_mock = server
        .mock("PUT", "/api/repos/alice/demo/landings/42/land")
        .with_status(409)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"landing request is not open"}"#)
        .create();

    let out = run_plue(
        &["land", "merge", "42", "-R", "alice/demo"],
        tmp.path(),
        &cfg_home,
    );

    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot be merged right now"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("landing request is not open"),
        "stderr: {stderr}"
    );
}
