//! Real synthetic native executables; no host account, provider, or model access.
use super::*;
use std::process::Command;
fn native_fixture(dir: &Path, kind: &str) -> PathBuf {
    let path = dir.join(if cfg!(windows) {
        format!("{kind}-fixture.exe")
    } else {
        format!("{kind}-fixture")
    });
    let source = dir.join(format!("{kind}.rs"));
    let code=r#"
use std::io::{BufRead,Write};
fn note(value:&str) {std::fs::OpenOptions::new().create(true).append(true).open(__LOG__).unwrap().write_all(format!("{}\n",value).as_bytes()).unwrap();}
fn main() {
 let a:Vec<String>=std::env::args().skip(1).collect();
 if __KIND__.starts_with("dreamina") {
  if a==["version"] {note("version");if __KIND__=="dreamina-version-failure" {std::process::exit(2);}if __KIND__=="dreamina-log-denied" {eprintln!("initialize logs failed: initialize file logger: open /PRIVATE_SENTINEL/.dreamina_cli/logs: operation not permitted");}println!("1.4.19");return;}
  if a.len()==2 && a[1]=="-h" && ["multimodal2video","text2video","text2image","image2image"].contains(&a[0].as_str()) {note(&format!("{} -h",a[0]));println!("official synthetic help only --model_version 5.0Pro --ratio 16:9 --resolution_type 2k");return;}
  if a==["user_credit"] {note("user_credit");println!("{{\"data\":{{\"user_id\":\"PRIVATE_SENTINEL\",\"total_credit\":42,\"gift_credit\":7}}}}");return;}
 } else {
  if a==["--version"] {note("version");if __KIND__=="codex-output-flood" {for _ in 0..2048 {std::io::stdout().write_all(&[b'x';1024]).unwrap();}return;}println!("codex-cli synthetic-readiness");return;}
  if a==["exec","--help"] {note("exec-help");println!("--json --output-schema --output-last-message --ignore-user-config --ignore-rules --ephemeral");return;}
  if a.first().map(String::as_str)==Some("app-server") {
   note("app-server");
   if __KIND__=="codex-rpc-failure" {note("rpc-unavailable");return;}
   if __KIND__=="codex-rpc-flood" {note("rpc-flood");for _ in 0..2048 {std::io::stdout().write_all(&[b'x';1024]).unwrap();}return;}
   for line in std::io::stdin().lock().lines() {
    let line=line.unwrap();
    if line.contains("\"initialize\"") {note("initialize");println!("{{\"id\":1,\"result\":{{}}}}");}
    else if line.contains("account/read") {assert!(line.contains("\"refreshToken\":false"));note("account/read:no-refresh");println!("{{\"id\":2,\"result\":{{\"account\":{{\"type\":\"chatgpt\",\"email\":\"PRIVATE_SENTINEL\",\"planType\":\"synthetic\"}}}}}}");}
    else if line.contains("model/list") {note("model/list");println!("{{\"id\":10,\"result\":{{\"data\":[{{\"model\":\"gpt-6.1-sol\",\"isDefault\":true,\"defaultReasoningEffort\":\"medium\",\"supportedReasoningEfforts\":[{{\"reasoningEffort\":\"medium\"}}]}}],\"nextCursor\":null}}}}");}
    else if line.contains("account/rateLimits/read") {note("account/rateLimits/read");println!("{{\"id\":30,\"result\":{{\"rateLimitsByLimitId\":{{}}}}}}");}
    else if !line.contains("\"initialized\"") {note("FORBIDDEN_RPC");std::process::exit(92);}
    std::io::stdout().flush().unwrap();
   }
   return;
  }
 }
 note("FORBIDDEN_COMMAND");std::process::exit(91);
}
"#.replace("__LOG__",&json!(dir.join(format!("{kind}-calls.txt"))).to_string()).replace("__KIND__",&json!(kind).to_string());
    fs::write(&source, code).unwrap();
    let compiler = std::env::var_os("RUSTC")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("CARGO_HOME")
                .map(PathBuf::from)
                .map(|home| {
                    home.join("bin")
                        .join(if cfg!(windows) { "rustc.exe" } else { "rustc" })
                })
                .filter(|path| path.is_file())
        })
        .unwrap_or_else(|| PathBuf::from(if cfg!(windows) { "rustc.exe" } else { "rustc" }));
    let output = Command::new(compiler)
        .args(["--edition=2021", "--crate-name", "readiness_fixture"])
        .arg(&source)
        .arg("-o")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    path
}
fn enabled_engine(home: PathBuf) -> Arc<Engine> {
    let mut engine = Engine::new(home).unwrap();
    // Test-only private state; no product flag, environment variable or IPC enables this.
    Arc::get_mut(&mut engine).unwrap().readiness_enabled = true;
    engine
}
#[test]
fn explicit_codex_check_uses_only_read_rpc_preserves_settings_and_never_unlocks_generation() {
    let dir = tempfile::tempdir().unwrap();
    let path = native_fixture(dir.path(), "codex");
    let engine = enabled_engine(dir.path().join("home"));
    let project = engine
        .dispatch(json!({"action":"projects.create","title":"synthetic readiness"}))
        .unwrap();
    let store = engine.store(s(&project, "id")).unwrap();
    let before = store.settings().unwrap();
    let result=engine.dispatch(json!({"action":"readiness.check","projectId":project["id"],"provider":"codex_cli","settings":{"codexPath":path}})).unwrap();
    assert_eq!(result["schemaVersion"], crate::readiness::VERSION);
    assert_eq!(result["installation"]["state"], "ready");
    assert_eq!(result["protocol"]["state"], "ready");
    assert_eq!(result["authentication"]["state"], "authenticated");
    assert_eq!(result["catalog"]["state"], "available");
    assert_eq!(result["network"]["state"], "not_tested");
    assert_eq!(result["generation"]["calls"], 0);
    assert!(!result.to_string().contains("PRIVATE_SENTINEL"));
    assert_eq!(store.settings().unwrap(), before);
    assert!(store.jobs().unwrap().is_empty());
    let policy = engine
        .dispatch(json!({"action":"getExecutionPolicy"}))
        .unwrap();
    assert_eq!(policy["liveExecutionAllowed"], false);
    assert_eq!(policy["readiness"]["enabled"], true);
    let log = fs::read_to_string(dir.path().join("codex-calls.txt")).unwrap();
    assert!(log.contains("account/read:no-refresh"));
    assert!(log.contains("model/list"));
    assert!(!log.contains("FORBIDDEN"));
    assert!(engine
        .dispatch(json!({"action":"foundation.run","projectId":project["id"]}))
        .unwrap_err()
        .contains("OFFLINE_EXECUTION_BLOCKED"));
    assert_eq!(
        fs::read_to_string(dir.path().join("codex-calls.txt")).unwrap(),
        log
    );
}
#[test]
fn dreamina_check_does_not_poll_oauth_resume_tasks_cache_catalogs_or_claim_free_images() {
    let dir = tempfile::tempdir().unwrap();
    let path = native_fixture(dir.path(), "dreamina");
    let engine = enabled_engine(dir.path().join("home"));
    let project = engine
        .dispatch(json!({"action":"projects.create","title":"synthetic readiness"}))
        .unwrap();
    let store = engine.store(s(&project, "id")).unwrap();
    let job = store
        .add_job(
            "image",
            "synthetic",
            "dreamina",
            "original-account",
            &json!({"prompt":"synthetic"}),
            "auth_required",
        )
        .unwrap();
    let before = store.job(&job).unwrap();
    let settings = store.settings().unwrap();
    let result=engine.dispatch(json!({"action":"readiness.check","projectId":project["id"],"provider":"dreamina","settings":{"dreaminaPath":path}})).unwrap();
    assert_eq!(result["authentication"]["state"], "authenticated");
    assert_eq!(result["details"]["credits"]["total"], 42.0);
    assert_eq!(result["details"]["freeImageEligibility"], "unverified");
    assert_eq!(result["generation"]["calls"], 0);
    assert!(!result.to_string().contains("PRIVATE_SENTINEL"));
    assert_eq!(store.job(&job).unwrap(), before);
    assert_eq!(store.settings().unwrap(), settings);
    let log = fs::read_to_string(dir.path().join("dreamina-calls.txt")).unwrap();
    assert_eq!(log.lines().filter(|l| *l == "user_credit").count(), 1);
    assert!(!log.contains("FORBIDDEN"));
    assert!(log.lines().all(|l| [
        "version",
        "multimodal2video -h",
        "text2video -h",
        "text2image -h",
        "image2image -h",
        "user_credit"
    ]
    .contains(&l)));
}
#[test]
fn isolated_startup_and_unsupported_readiness_requests_fail_before_any_probe() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::new_offline_verification(dir.path().join("probe")).unwrap();
    let policy = engine
        .dispatch(json!({"action":"getExecutionPolicy"}))
        .unwrap();
    assert_eq!(policy["readiness"]["enabled"], false);
    assert!(engine
        .dispatch(
            json!({"action":"readiness.check","provider":"codex_cli","projectId":"does-not-exist"})
        )
        .unwrap_err()
        .contains("READINESS_NATIVE_ONLY"));
    for provider in ["api", "bh", "unknown"] {
        assert!(
            crate::readiness::request_settings(&json!({"provider":provider}), &json!({})).is_err()
        );
    }
    for field in [
        "textApiKey",
        "sdkOfflineFixture",
        "execute",
        "login",
        "resourceRoot",
    ] {
        assert!(crate::readiness::request_settings(
            &json!({"provider":"codex_sdk","settings":{field:"forbidden"}}),
            &json!({})
        )
        .is_err());
    }
}

#[test]
fn rpc_failure_keeps_verified_cli_version_and_does_not_claim_logged_out_or_model_access() {
    let dir = tempfile::tempdir().unwrap();
    let path = native_fixture(dir.path(), "codex-rpc-failure");
    let engine = enabled_engine(dir.path().join("home"));
    let p = engine
        .dispatch(json!({"action":"projects.create","title":"synthetic RPC failure"}))
        .unwrap();
    let r=engine.dispatch(json!({"action":"readiness.check","provider":"codex_cli","projectId":p["id"],"settings":{"codexPath":path}})).unwrap();
    assert_eq!(r["installation"]["state"], "ready");
    assert_eq!(
        r["installation"]["version"],
        "codex-cli synthetic-readiness"
    );
    assert_eq!(r["protocol"]["state"], "failed");
    assert_eq!(r["authentication"]["state"], "unknown");
    assert_eq!(r["catalog"]["state"], "unavailable");
    assert_eq!(r["generation"]["calls"], 0);
    assert!(engine
        .store(s(&p, "id"))
        .unwrap()
        .jobs()
        .unwrap()
        .is_empty());
}

#[test]
fn selected_cli_output_floods_fail_bounded_without_account_or_generation_claims() {
    for kind in ["codex-output-flood", "codex-rpc-flood"] {
        let dir = tempfile::tempdir().unwrap();
        let path = native_fixture(dir.path(), kind);
        let engine = enabled_engine(dir.path().join("home"));
        let p = engine
            .dispatch(json!({"action":"projects.create","title":"synthetic oversized CLI"}))
            .unwrap();
        let start = std::time::Instant::now();
        let r=engine.dispatch(json!({"action":"readiness.check","provider":"codex_cli","projectId":p["id"],"settings":{"codexPath":path}})).unwrap();
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        assert_eq!(r["protocol"]["state"], "failed");
        assert_eq!(r["authentication"]["state"], "unknown");
        assert_eq!(r["generation"]["calls"], 0);
        assert!(
            r["messages"]
                .to_string()
                .contains(if kind == "codex-output-flood" {
                    "CLI_INSPECTION_OUTPUT_LIMIT"
                } else {
                    "CLI_RPC_LINE_LIMIT"
                }),
            "{r}"
        );
        assert!(r.to_string().len() < 8192);
        assert!(engine
            .store(s(&p, "id"))
            .unwrap()
            .jobs()
            .unwrap()
            .is_empty());
        let log = fs::read_to_string(dir.path().join(format!("{kind}-calls.txt"))).unwrap();
        assert!(!log.contains("FORBIDDEN"));
        assert!(!log.contains("account/read"));
    }
}

#[test]
fn dreamina_log_write_denial_stops_further_probes_and_does_not_claim_auth_failure() {
 let dir=tempfile::tempdir().unwrap();let path=native_fixture(dir.path(),"dreamina-log-denied");
 let engine=enabled_engine(dir.path().join("home"));let p=engine.dispatch(json!({"action":"projects.create","title":"synthetic logger denial"})).unwrap();
 let r=engine.dispatch(json!({"action":"readiness.check","provider":"dreamina","projectId":p["id"],"settings":{"dreaminaPath":path}})).unwrap();
 assert_eq!(r["details"]["localEnvironmentIssue"],"CLI_LOCAL_LOG_WRITE_RESTRICTED");assert_eq!(r["authentication"]["state"],"not_checked");assert_eq!(r["protocol"]["state"],"failed");assert!(!r.to_string().contains("PRIVATE_SENTINEL"));
 assert_eq!(fs::read_to_string(dir.path().join("dreamina-log-denied-calls.txt")).unwrap(),"version\n");
 assert!(engine.store(s(&p,"id")).unwrap().jobs().unwrap().is_empty());
}

fn canvas_native_fixture(dir: &Path, kind: &str) -> PathBuf {
    use crate::readiness_canvas::ReadCommand;
    let path=dir.join(if cfg!(windows) {format!("{kind}.exe")} else {kind.into()});
    let mut arms=String::new();
    for (command, response) in [
        (ReadCommand::Version,include_str!("../../fixtures/synthetic/canvas-readiness/version.json")),
        (ReadCommand::Schema,include_str!("../../fixtures/synthetic/canvas-readiness/schema.json")),
        (ReadCommand::AuthStatus,include_str!("../../fixtures/synthetic/canvas-readiness/auth-status.json")),
        (ReadCommand::AuthAccount,include_str!("../../fixtures/synthetic/canvas-readiness/account.json")),
        (ReadCommand::ImageModels,include_str!("../../fixtures/synthetic/canvas-readiness/models.json")),
    ] {
        let response=if kind=="canvas-output-flood" && command==ReadCommand::Version {"x".repeat(70_000)} else {response.into()};
        arms.push_str(&format!("{} => println!(\"{{}}\", {}),\n",json!(command.args().join(" ")),json!(response)));
    }
    // The old readiness path invokes `version` without globals; emit public Canvas JSON
    // so the discriminator can stop before any legacy helper or account command.
    arms.push_str(&format!("\"version\" => println!(\"{{}}\", {}),\n",json!(include_str!("../../fixtures/synthetic/canvas-readiness/version.json"))));
    let code=format!(r#"use std::io::Write;
fn main() {{ let args=std::env::args().skip(1).collect::<Vec<_>>().join(" ");
let mut log=std::fs::OpenOptions::new().create(true).append(true).open({}).unwrap(); writeln!(log,"{{}}",args).unwrap();
match args.as_str() {{ {} _ => {{ writeln!(log,"FORBIDDEN").unwrap(); std::process::exit(91); }} }} }}"#,json!(dir.join(format!("{kind}-calls.txt"))),arms);
    let source=dir.join(format!("{kind}.rs"));fs::write(&source,code).unwrap();
    let compiler=std::env::var_os("RUSTC").map(PathBuf::from).or_else(||std::env::var_os("CARGO_HOME").map(PathBuf::from).map(|home|home.join("bin").join(if cfg!(windows){"rustc.exe"}else{"rustc"}))).unwrap_or_else(||PathBuf::from("rustc"));
    let output=Command::new(compiler).args(["--edition=2021","--crate-name","canvas_readiness_fixture"]).arg(&source).arg("-o").arg(&path).output().unwrap();
    assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));path
}

#[test]
fn canvas_readiness_native_allowlist_preserves_legacy_jobs_settings_and_generation_gates() {
    let dir=tempfile::tempdir().unwrap();let path=canvas_native_fixture(dir.path(),"canvas-success");let engine=enabled_engine(dir.path().join("home"));
    let project=engine.dispatch(json!({"action":"projects.create","title":"Synthetic Canvas readiness"})).unwrap();let store=engine.store(s(&project,"id")).unwrap();
    store.save_settings(&json!({"dreaminaPath":"/legacy/unchanged","imageProvider":"dreamina","imageModel":"5.0"})).unwrap();
    let job=store.add_job("image","synthetic","dreamina","original-account",&json!({"prompt":"synthetic"}),"auth_required").unwrap();
    let settings=store.settings().unwrap();let before=store.job(&job).unwrap();
    let report=engine.dispatch(json!({"action":"readiness.check","projectId":project["id"],"provider":"dreamina_canvas","settings":{"dreaminaCanvasPath":path}})).unwrap();
    assert_eq!(report["installation"]["version"],"1.0.2");assert_eq!(report["protocol"]["state"],"ready");assert_eq!(report["authentication"]["identityVerified"],true);assert_eq!(report["catalog"]["models"].as_array().unwrap().len(),3);
    assert_eq!(report["generation"]["calls"],0);assert_eq!(report["details"]["quota"]["remaining"],Value::Null);assert!(!report.to_string().contains("PRIVATE_"));assert!(!report.to_string().contains("123456789"));
    assert_eq!(store.settings().unwrap(),settings);assert_eq!(store.job(&job).unwrap(),before);
    let log=fs::read_to_string(dir.path().join("canvas-success-calls.txt")).unwrap();assert_eq!(log.lines().count(),5);assert!(!log.contains("FORBIDDEN"));
    for forbidden in ["login","logout","user_credit","--yes","node","create","quote","run"] {assert!(!log.split_whitespace().any(|arg|arg==forbidden));}
    assert!(engine.dispatch(json!({"action":"images.run","projectId":project["id"]})).unwrap_err().contains("OFFLINE_EXECUTION_BLOCKED"));
    assert_eq!(fs::read_to_string(dir.path().join("canvas-success-calls.txt")).unwrap(),log);
    for field in ["dreaminaPath","login","command","profile","region","authToken","sdkOfflineFixture"] {
        assert!(crate::readiness::request_settings(&json!({"provider":"dreamina_canvas","settings":{field:"forbidden"}}),&json!({})).is_err());
    }
}

#[test]
fn canvas_selected_in_legacy_readiness_stops_before_any_legacy_account_or_help() {
    let dir=tempfile::tempdir().unwrap();let path=canvas_native_fixture(dir.path(),"canvas-as-legacy");let engine=enabled_engine(dir.path().join("home"));
    let p=engine.dispatch(json!({"action":"projects.create","title":"Synthetic wrong protocol"})).unwrap();
    let report=engine.dispatch(json!({"action":"readiness.check","projectId":p["id"],"provider":"dreamina","settings":{"dreaminaPath":path}})).unwrap();
    assert_eq!(report["protocol"]["state"],"unsupported");assert_eq!(report["authentication"]["state"],"not_checked");
    assert_eq!(fs::read_to_string(dir.path().join("canvas-as-legacy-calls.txt")).unwrap(),"version\n");
}

#[test]
fn canvas_oversize_version_stops_before_schema_or_account_and_missing_path_never_falls_back() {
    let dir=tempfile::tempdir().unwrap();let path=canvas_native_fixture(dir.path(),"canvas-output-flood");let engine=enabled_engine(dir.path().join("home"));
    let p=engine.dispatch(json!({"action":"projects.create","title":"Synthetic bounded Canvas"})).unwrap();
    let report=engine.dispatch(json!({"action":"readiness.check","projectId":p["id"],"provider":"dreamina_canvas","settings":{"dreaminaCanvasPath":path}})).unwrap();
    assert_eq!(report["protocol"]["state"],"failed");assert!(report.to_string().contains("CANVAS_OUTPUT_LIMIT"));assert!(report.to_string().len()<8192);
    assert_eq!(fs::read_to_string(dir.path().join("canvas-output-flood-calls.txt")).unwrap(),"--format json version\n");
    let missing=crate::readiness_canvas::check(&json!({"dreaminaPath":path}));assert_eq!(missing["installation"]["state"],"missing");
    assert_eq!(fs::read_to_string(dir.path().join("canvas-output-flood-calls.txt")).unwrap(),"--format json version\n");
}

#[test]
fn failed_legacy_version_never_reaches_user_credit() {
    let dir=tempfile::tempdir().unwrap();let path=native_fixture(dir.path(),"dreamina-version-failure");
    let engine=enabled_engine(dir.path().join("home"));let project=engine.dispatch(json!({"action":"projects.create","title":"Synthetic legacy version failure"})).unwrap();
    let report=engine.dispatch(json!({"action":"readiness.check","projectId":project["id"],"provider":"dreamina","settings":{"dreaminaPath":path}})).unwrap();
    assert_eq!(report["protocol"]["state"],"failed");assert_eq!(report["authentication"]["state"],"not_checked");
    assert_eq!(fs::read_to_string(dir.path().join("dreamina-version-failure-calls.txt")).unwrap(),"version\n");
}
