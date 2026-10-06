use director_core::{execution_policy, pipeline::TextRequest, platform, text_transport::*, Engine};
use serde_json::json;

#[test]
fn source_import_roundtrips_in_new_app_and_external_actions_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("next");
    let engine = Engine::new(home.clone()).unwrap();
    let policy = engine
        .dispatch(json!({"action":"getExecutionPolicy"}))
        .unwrap();
    assert_eq!(policy["liveExecutionAllowed"], false);
    let project = engine
        .dispatch(json!({"action":"projects.create","title":"合成离线项目"}))
        .unwrap();
    let id = project["id"].clone();
    let source = "【故事简介】\n两位同名人物在不同年代相遇。\n第1集\n场景：门口\n甲（OS）：等一下🙂。\n第2集\n场景：庭院\n甲：我回来了。\n";
    let workspace = engine
        .dispatch(json!({"action":"source.import","projectId":id,"text":source}))
        .unwrap();
    assert_eq!(workspace["source"]["text"], source);
    assert_eq!(workspace["source"]["episodes"].as_array().unwrap().len(), 2);
    let original = workspace["source"].clone();
    for action in [
        "foundation.run",
        "storyboard.run",
        "images.run",
        "videos.prepare",
        "videos.submit",
        "providers.refresh",
        "textCli.status",
        "textCli.login",
        "dreamina.status",
        "jobs.query",
        "unknown.action",
    ] {
        let error = engine
            .dispatch(json!({"action":action,"projectId":id}))
            .unwrap_err();
        assert!(
            error.starts_with("OFFLINE_EXECUTION_BLOCKED:"),
            "{action}: {error}"
        );
    }
    engine.tick().unwrap();
    assert!(engine
        .store(id.as_str().unwrap())
        .unwrap()
        .jobs()
        .unwrap()
        .is_empty());
    drop(engine);
    let reopened = Engine::new(home).unwrap();
    let workspace = reopened
        .dispatch(json!({"action":"workspace.get","projectId":id}))
        .unwrap();
    assert_eq!(workspace["source"], original);
}
#[test]
fn old_registry_is_not_adopted_or_mutated() {
    let dir = tempfile::tempdir().unwrap();
    let original = b"[{\"id\":\"old-private-project\"}]";
    std::fs::write(dir.path().join("projects.json"), original).unwrap();
    assert!(Engine::new(dir.path().to_path_buf()).is_err());
    assert_eq!(
        std::fs::read(dir.path().join("projects.json")).unwrap(),
        original
    );
    assert!(!dir.path().join(platform::HOME_MARKER).exists());
}
#[test]
fn nonsecret_settings_are_allowed_but_credentials_and_unknown_actions_are_denied() {
    assert!(execution_policy::check(
        &json!({"action":"settings.save","settings":{"textModel":"gpt-6.1-sol"}})
    )
    .is_ok());
    for field in execution_policy::CREDENTIAL_FIELDS {
        assert!(execution_policy::check(
            &json!({"action":"settings.save","settings":{*field:"synthetic-secret"}})
        )
        .is_err());
    }
    assert!(execution_policy::check(
        &json!({"action":"settings.save","settings":{"textApiKey":42}})
    )
    .is_err());
}
#[test]
fn media_scope_excludes_database_and_rejects_escaped_symlink() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("media")).unwrap();
    std::fs::write(dir.path().join("project.sqlite"), b"synthetic").unwrap();
    let scopes = platform::media_scopes(dir.path()).unwrap();
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].file_name().unwrap(), "media");
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("thumbnails")).unwrap();
        assert!(platform::media_scopes(dir.path()).is_err());
    }
}
fn attempt(backend: Backend) -> FrozenAttempt {
    FrozenAttempt::prepare(
        "attempt-1",
        "workflow-1",
        "node-1",
        backend,
        TextRequest {
            stage: "storyboardCreate".into(),
            skill: "synthetic-only".into(),
            model: "gpt-6.1-sol".into(),
            reasoning: "medium".into(),
            input: json!({"original":"甲：原话。","unresolved":["identity-unknown"]}),
            schema: json!({"type":"object"}),
            rule_snapshot: json!([]),
        },
        &json!({"version":"synthetic-runtime","permissions":"none"}),
    )
    .unwrap()
}
#[test]
fn frozen_transport_identity_blocks_cross_backend_retry_and_survives_restart() {
    let mut a = attempt(Backend::CodexCli);
    let sdk = attempt(Backend::CodexSdk);
    assert_eq!(a.semantic_request_hash, sdk.semantic_request_hash);
    assert!(!a.has_same_execution_identity(&sdk));
    assert!(a.begin_dispatch(Backend::CodexSdk).is_err());
    a.begin_dispatch(Backend::CodexCli).unwrap();
    let unknown = Observation {
        attempt_id: a.attempt_id.clone(),
        sequence: 1,
        kind: ObservationKind::SubmissionUnknown,
        thread_id: Some("thread-original".into()),
        turn_id: Some("turn-original".into()),
        raw_result: None,
    };
    assert!(a.observe(unknown.clone()).unwrap());
    assert!(!a.observe(unknown).unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("attempt.json");
    a.persist(&path).unwrap();
    let mut loaded = FrozenAttempt::load(&path).unwrap();
    assert_eq!(loaded.thread_id.as_deref(), Some("thread-original"));
    assert!(loaded.begin_dispatch(Backend::CodexCli).is_err());
    assert!(loaded
        .observe(Observation {
            attempt_id: loaded.attempt_id.clone(),
            sequence: 2,
            kind: ObservationKind::Completed,
            thread_id: Some("other-thread".into()),
            turn_id: None,
            raw_result: Some(json!({}))
        })
        .is_err());
    assert_eq!(loaded.state, AttemptState::SubmissionUnknown);
}
#[tokio::test]
async fn sdk_stub_reports_unavailable_and_never_falls_back() {
    let sdk = UnavailableSdkTransport;
    assert_eq!(sdk.inspect().await.unwrap()["available"], false);
    assert!(sdk
        .execute(&attempt(Backend::CodexSdk))
        .await
        .unwrap_err()
        .contains("SDK_NOT_IMPLEMENTED"));
}

#[tokio::test]
async fn unsupported_selected_backend_never_falls_back_or_creates_a_job() {
    use director_core::{pipeline::TextExecutor, store::Store, text_executor::SavedTextExecutor};
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("project")).unwrap();
    store
        .create("synthetic backend selection", "selection")
        .unwrap();
    for provider in ["codex_sdk", "unrecognized-backend"] {
        let executor = SavedTextExecutor {
            store: store.clone(),
            settings: json!({"textProvider":provider}),
            workflow_id: None,
        };
        let error = executor
            .execute(attempt(Backend::CodexSdk).request)
            .await
            .unwrap_err();
        assert!(error.contains("不回退 CLI"));
        assert!(store.jobs().unwrap().is_empty());
    }
}

#[test]
fn tampered_frozen_payload_cannot_load_dispatch_or_claim_compatible_identity() {
    let a = attempt(Backend::CodexCli);
    let mut tampered = a.clone();
    tampered.request.input["original"] = json!("重写原话");
    assert!(tampered.begin_dispatch(Backend::CodexCli).is_err());
    assert!(!tampered.has_same_execution_identity(&a));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("attempt.json");
    std::fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(FrozenAttempt::load(&path).is_err());
    let mut backend = a.clone();
    backend.backend = Backend::CodexSdk;
    assert!(backend.validate_identity().is_err());
}

#[test]
fn previous_dispatch_cannot_be_reset_to_prepared_or_rebound_to_another_workflow() {
    let mut a = attempt(Backend::CodexCli);
    a.begin_dispatch(Backend::CodexCli).unwrap();
    a.state = AttemptState::Prepared;
    assert!(a.begin_dispatch(Backend::CodexCli).is_err());
    for evidence in 0..4 {
        let mut a = attempt(Backend::CodexCli);
        match evidence {
            0 => a.thread_id = Some("old-thread".into()),
            1 => a.turn_id = Some("old-turn".into()),
            2 => a.last_sequence = 1,
            _ => a.raw_result = Some(json!({"old":"raw"})),
        }
        assert!(a.begin_dispatch(Backend::CodexCli).is_err());
    }
    let mut a = attempt(Backend::CodexCli);
    a.workflow_id = "different-workflow".into();
    assert!(a.validate_identity().is_err());
    let mut a = attempt(Backend::CodexCli);
    a.attempt_id = "different-attempt".into();
    assert!(a.validate_identity().is_err());
    let mut a = attempt(Backend::CodexCli);
    a.logical_node_id = "different-node".into();
    assert!(a.validate_identity().is_err());
}
#[test]
fn disk_envelope_rejects_state_rollback_even_when_all_receipt_fields_are_removed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("attempt.json");
    let mut a = attempt(Backend::CodexCli);
    a.begin_dispatch(Backend::CodexCli).unwrap();
    a.persist(&path).unwrap();
    let mut disk: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    disk["payload"]["state"] = json!("prepared");
    disk["payload"]["dispatchStarted"] = json!(false);
    std::fs::write(&path, serde_json::to_vec(&disk).unwrap()).unwrap();
    assert!(FrozenAttempt::load(&path).is_err());
}
