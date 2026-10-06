//! Synthetic Stage2 design/publication/restart regression. Never a live model call.
use async_trait::async_trait;
use director_core::{
    pipeline::{self, TextExecutor, TextRequest},
    store::Store,
};
use serde_json::{json, Value};
use std::sync::Mutex;
#[derive(Default)]
struct OfflineDesigner {
    calls: Mutex<Vec<TextRequest>>,
}
#[async_trait]
impl TextExecutor for OfflineDesigner {
    async fn execute(&self, r: TextRequest) -> Result<Value, String> {
        self.calls.lock().unwrap().push(r.clone());
        if r.stage == "assetVisualFacts" {
            return Ok(
                json!({"schemaVersion":"asset-visual-facts-wire/v1","coveredPacketRef":r.input["coveredPacketRef"],"facts":[],"issues":[]}),
            );
        }
        assert_eq!(r.stage, "assetDesign");
        let a = &r.input["asset"];
        Ok(
            json!({"schemaVersion":"asset-design/v1","assetId":a["id"],"variantId":a["variantId"],"prompt":"六十岁深肤色人物，宽颧颊与舒展眉眼、朴素棉麻衣装；招牌写第十集。","freeDesignScope":"仅补足未限定脸部结构和衣装剪裁","sourceRefs":a["sourceRefs"],"referenceVariantIds":r.input["parentAsset"]["id"].as_str().map(|id|vec![id]).unwrap_or_default()}),
        )
    }
}
fn input(store: &Store) -> Value {
    json!({"source":store.source().unwrap(),"foundation":store.foundation().unwrap(),"assets":store.values("assets").unwrap(),"settings":store.settings().unwrap(),"imageProductionPolicy":"automatic"})
}
#[tokio::test]
async fn stage2_receipts_survive_publication_restart_and_style_refresh_keeps_fact_cache() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    let store = Store::open(root.clone()).unwrap();
    store.create("synthetic", "synthetic").unwrap();
    let source = store
        .save_source(json!(pipeline::prepare_source(
            "第1集\n场景：门口\n△甲穿棉衣，后来外衣破损。\n"
        )
        .unwrap()))
        .unwrap();
    let row = &source["rows"][2]["id"];
    store.save_settings(&json!({"projectVisualStyle":"水墨国漫","ratio":"3:2","productionDirection":"朴素材质完成度","productionDirectionConfirmed":true})).unwrap();
    for asset in [
        json!({"id":"a","baseAssetId":"person","variantId":"base","kind":"CH","sourceRefs":[row]}),
        json!({"id":"b","baseAssetId":"person","variantId":"hurt","kind":"CH","sourceAssetId":"a","sourceRefs":[row]}),
    ] {
        store.save_asset(asset).unwrap();
    }
    let exec = OfflineDesigner::default();
    let first = pipeline::run_asset_design(&input(&store), &exec, "offline", "medium")
        .await
        .unwrap();
    assert!(first["blocked"].as_array().unwrap().is_empty(), "{first}");
    assert_eq!(first["assets"].as_array().unwrap().len(), 2);
    for request in exec
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|r| r.stage == "assetDesign")
    {
        assert_eq!(request.input["projectVisualStyle"], "水墨国漫");
        assert_eq!(request.input["productionDirection"], "朴素材质完成度");
        assert_eq!(request.input["imagePresentation"]["ratio"], "3:2");
        assert!(request.input["comparisonDesigns"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    store.publish_asset_design(1, 0, &first).unwrap();
    let reopened = Store::open(root).unwrap();
    let after = input(&reopened);
    for asset in after["assets"].as_array().unwrap() {
        assert!(
            !pipeline::automatic_design_required(&after, asset),
            "receipt stale after publication: {asset}"
        );
    }
    let reuse = OfflineDesigner::default();
    let second = pipeline::run_asset_design(&after, &reuse, "offline", "medium")
        .await
        .unwrap();
    assert!(second["blocked"].as_array().unwrap().is_empty(), "{second}");
    assert!(reuse.calls.lock().unwrap().is_empty());
    reopened
        .save_settings(&json!({"projectVisualStyle":"细腻CG国漫"}))
        .unwrap();
    let refresh = OfflineDesigner::default();
    let third = pipeline::run_asset_design(&input(&reopened), &refresh, "offline", "medium")
        .await
        .unwrap();
    assert!(third["blocked"].as_array().unwrap().is_empty(), "{third}");
    assert_eq!(third["assets"].as_array().unwrap().len(), 2);
    assert_eq!(
        refresh
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.stage == "assetVisualFacts")
            .count(),
        0,
        "style must not invalidate a visual-fact cache"
    );
    assert_eq!(
        reopened.get("assets", "a").unwrap()["design"],
        after["assets"][0]["design"],
        "an unadopted refresh never rewrites saved history"
    );
}
#[tokio::test]
async fn stage2_publication_rejects_concurrent_direct_source_edit_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path().join("project")).unwrap();
    store.create("synthetic", "synthetic").unwrap();
    let source = store
        .save_source(json!(pipeline::prepare_source(
            "第1集\n场景：门口\n△甲衣服破损。\n"
        )
        .unwrap()))
        .unwrap();
    let row = &source["rows"][2]["id"];
    store.save_asset(json!({"id":"a","variantId":"base","kind":"CH","prompt":"指定手写来源","promptOrigin":"MANUAL","sourceRefs":[row]})).unwrap();
    store
        .save_asset(
            json!({"id":"b","variantId":"hurt","kind":"CH","sourceAssetId":"a","sourceRefs":[row]}),
        )
        .unwrap();
    let mut request = input(&store);
    request["selection"] = json!({"assetIds":["b"]});
    let output =
        pipeline::run_asset_design(&request, &OfflineDesigner::default(), "offline", "medium")
            .await
            .unwrap();
    assert!(output["blocked"].as_array().unwrap().is_empty(), "{output}");
    let mut edited = store.get("assets", "a").unwrap();
    edited["prompt"] = json!("后来改动来源衣装");
    store.save_asset(edited).unwrap();
    assert!(store
        .publish_asset_design(1, 0, &output)
        .unwrap_err()
        .contains("来源语义版本"));
    assert!(store.get("assets", "b").unwrap()["prompt"].is_null());
    assert!(store.foundation().unwrap().is_null());
}

#[test]
fn interrupted_stage2_text_resumes_original_workflow_without_extra_calls() {
    use director_core::{execution_policy::OfflineChannels, Engine};
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let engine = Engine::with_channels(home.clone(), Arc::new(OfflineChannels)).unwrap();
    let project = engine
        .dispatch(json!({"action":"projects.create","title":"Synthetic Stage2 resume"}))
        .unwrap();
    let store = engine.store(project["id"].as_str().unwrap()).unwrap();
    let source = store
        .save_source(json!(pipeline::prepare_source(
            "第1集\n场景：门口\n△甲穿棉衣。\n"
        )
        .unwrap()))
        .unwrap();
    store.save_asset(json!({"id":"manual","variantId":"base","kind":"CH","prompt":"明确手写棉衣造型","promptOrigin":"MANUAL","sourceRefs":[source["rows"][2]["id"]]})).unwrap();
    store
        .save_settings(&json!({"projectVisualStyle":"original-frozen-style"}))
        .unwrap();
    let mut context = input(&store);
    context["selection"] = json!({"assetIds":["manual"]});
    context["designRuleSha256"] =
        pipeline::rule_snapshot("asset-art-director").unwrap()["sha256"].clone();
    let snapshot = json!({"runnerVersion":"stage2-runner/v1","stage":"images.design","sourceVersion":source["version"],"settings":store.settings().unwrap(),"request":{"action":"images.design","projectId":project["id"],"assetIds":["manual"]},"imageProductionPolicy":"automatic","stage2Context":context});
    let workflow = store
        .add_job(
            "workflow",
            "images.design:",
            "local",
            "local",
            &snapshot,
            "interrupted",
        )
        .unwrap();
    store
        .save_settings(&json!({"projectVisualStyle":"changed-current-style"}))
        .unwrap();
    drop(engine);
    let reopened = Engine::with_channels(home, Arc::new(OfflineChannels)).unwrap();
    reopened
        .dispatch(
            json!({"action":"workflows.resume","projectId":project["id"],"workflowId":workflow}),
        )
        .expect("same Stage2 workflow must be resumable");
    assert!(reopened.await_worker_shutdown(10));
    let done = store.job(&workflow).unwrap();
    assert_eq!(done["status"], "completed", "{done}");
    assert_eq!(done["snapshot"], snapshot);
    let jobs = store.jobs().unwrap();
    assert_eq!(
        jobs.len(),
        1,
        "no fresh workflow/text/media job or model call"
    );
    assert_eq!(
        store.get("assets", "manual").unwrap()["prompt"],
        "明确手写棉衣造型"
    );
    assert!(reopened
        .dispatch(
            json!({"action":"workflows.resume","projectId":project["id"],"workflowId":workflow})
        )
        .is_err());
    assert_eq!(store.jobs().unwrap().len(), 1);
}

#[tokio::test]
async fn partial_stage2_output_holds_versions_and_success_commits_with_parent_atomically() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("project")).unwrap();
    store
        .create("Synthetic atomic Stage2", "synthetic")
        .unwrap();
    let source = store
        .save_source(json!(pipeline::prepare_source(
            "第1集\n场景：门口\n△甲穿棉衣。\n"
        )
        .unwrap()))
        .unwrap();
    store
        .save_asset(
            json!({"id":"a","variantId":"base","kind":"CH","sourceRefs":[source["rows"][2]["id"]]}),
        )
        .unwrap();
    let original_assets = store.values("assets").unwrap();
    let original_foundation = store.foundation().unwrap();
    let exec = OfflineDesigner::default();
    let clean = pipeline::run_asset_design(&input(&store), &exec, "synthetic", "medium")
        .await
        .unwrap();
    assert!(clean["blocked"].as_array().unwrap().is_empty());
    let calls = exec.calls.lock().unwrap().len();
    let snapshot = json!({"runnerVersion":"stage2-runner/v1","stage":"images.design","sourceVersion":source["version"]});
    let run = store
        .add_job(
            "workflow",
            "images.design:",
            "local",
            "local",
            &snapshot,
            "running",
        )
        .unwrap();
    let mut partial = clean.clone();
    partial["blocked"] = json!([{"assetId":"other","error":"synthetic interruption"}]);
    let held = store
        .finish_asset_design_workflow(1, 0, &partial, &run)
        .unwrap();
    assert_eq!(held["publication"]["status"], "held_partial");
    assert_eq!(store.job(&run).unwrap()["status"], "partially_blocked");
    assert_eq!(store.values("assets").unwrap(), original_assets);
    assert_eq!(store.foundation().unwrap(), original_foundation);
    store.update_job(&run, "running", &json!({})).unwrap();
    store.conn().unwrap().execute_batch("CREATE TRIGGER fail_parent_complete BEFORE UPDATE ON jobs WHEN NEW.state='completed' BEGIN SELECT RAISE(ABORT,'synthetic parent failure'); END;").unwrap();
    assert!(store
        .finish_asset_design_workflow(1, 0, &clean, &run)
        .unwrap_err()
        .contains("synthetic parent failure"));
    assert_eq!(
        store.values("assets").unwrap(),
        original_assets,
        "asset adoption must roll back with parent failure"
    );
    assert_eq!(
        store.foundation().unwrap(),
        original_foundation,
        "foundation must roll back with parent failure"
    );
    assert_eq!(store.job(&run).unwrap()["status"], "running");
    store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER fail_parent_complete;")
        .unwrap();
    let done = store
        .finish_asset_design_workflow(1, 0, &clean, &run)
        .unwrap();
    assert_eq!(done["publication"]["status"], "completed");
    let record = store.job(&run).unwrap();
    assert_eq!(record["status"], "completed");
    assert_eq!(record["snapshot"], snapshot);
    let durable:String=store.conn().unwrap().query_row("SELECT result FROM jobs WHERE id=?1",[&run],|r|r.get(0)).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&durable).unwrap(),done);
    assert_ne!(store.values("assets").unwrap(), original_assets);
    let adopted = store.values("assets").unwrap();
    let foundation = store.foundation().unwrap();
    assert!(store
        .finish_asset_design_workflow(1, 0, &clean, &run)
        .is_err());
    assert_eq!(store.values("assets").unwrap(), adopted);
    assert_eq!(store.foundation().unwrap(), foundation);
    assert_eq!(store.jobs().unwrap().len(), 1);
    assert_eq!(exec.calls.lock().unwrap().len(), calls);
}
