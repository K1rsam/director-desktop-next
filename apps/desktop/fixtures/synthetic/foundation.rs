// Entirely synthetic test builders. No saved production request or source is imported.
use super::*;

pub(crate) const CUT_LINES: [usize; 6] = [4, 12, 20, 28, 36, 44];

pub(crate) fn source() -> LockedSource {
    prepare_source(include_str!("foundation-source.txt")).unwrap()
}

fn batch(source: &LockedSource, scope: Vec<String>) -> Batch {
    Batch {
        pieces: whole_rows(source, &scope),
        scope,
        fragment: None,
    }
}

fn snapshot_for(
    stage: &str,
    source: &LockedSource,
    batch: &Batch,
    prior: &FoundationProposal,
) -> Value {
    let input = input_for(
        stage,
        source,
        batch,
        prior,
        &json!({}),
        0,
        1,
        stage == "Q1",
        &[],
    );
    json!({"stage":format!("foundation:{stage}"),"input":input,"schema":delivery_schema(stage, &input)})
}

pub(crate) fn catalogue() -> Value {
    let source = source();
    let batch = batch(
        &source,
        source.episodes.iter().map(|e| e.id.clone()).collect(),
    );
    let snapshot = snapshot_for("F2", &source, &batch, &empty_foundation(&source));
    let group = format!(
        "row:{}:{}:{}",
        &source.sha256[..12],
        source.rows[2].range.start,
        source.rows[3].range.end
    );
    let mut assets: Vec<_> = (0..47)
        .map(|i| {
            json!({
                "name":format!("合成测试道具{i:02}"), "type":"PROP",
                "episodes":[source.episodes[0].id],
                "sceneRefs":[[source.episodes[0].id,source.scenes[0].id]],
                "sourceRefs":[group], "preparation":{"mode":"REFERENCE"}
            })
        })
        .collect();
    // An omitted child must survive the later keyed base-only patch unchanged.
    assets[0]["variants"] = json!([{
        "name":"合成测试备用形象", "episodes":[source.episodes[0].id],
        "sceneRefs":[[source.episodes[0].id,source.scenes[0].id]],
        "preparation":{"mode":"REFERENCE"}
    }]);
    json!({"snapshot":snapshot,"result":{"format":"ASSET_CATALOG_DESKTOP_V2","sourceSha256":source.sha256,"coverage":batch.scope,"assets":assets}})
}

fn accepted_catalogue(source: &LockedSource) -> FoundationProposal {
    let fixture = catalogue();
    let input = &fixture["snapshot"]["input"];
    let batch = batch(
        source,
        serde_json::from_value(input["scope"].clone()).unwrap(),
    );
    let mut raw = fixture["result"].clone();
    normalize_legacy_catalogue(&mut raw, input).unwrap();
    apply_catalogue(
        &raw,
        source,
        &batch,
        input,
        &empty_foundation(source),
        None,
        &HashSet::new(),
    )
    .unwrap()
}

pub(crate) fn editor() -> Value {
    let source = source();
    let prior = accepted_catalogue(&source);
    let batch = batch(
        &source,
        source.episodes[..6].iter().map(|e| e.id.clone()).collect(),
    );
    let snapshot = snapshot_for("Q1", &source, &batch, &prior);
    let splits: Vec<_> = CUT_LINES
        .iter()
        .map(|line| {
            let row = source.rows.iter().find(|r| r.number == *line).unwrap();
            json!({"episodeId":row.episode_id,"sceneId":row.scene_id,"cuts":[{
                "rowId":format!("r:{}:{}",row.range.start,row.range.end),
                "anchor":row.text.trim_end_matches(['\r','\n'])
            }]})
        })
        .collect();
    let row = source
        .rows
        .iter()
        .find(|r| r.number == CUT_LINES[5])
        .unwrap();
    let child = format!("{}__L{}", row.scene_id.as_ref().unwrap(), row.number);
    let metadata_row = source
        .rows
        .iter()
        .find(|r| r.number == CUT_LINES[1])
        .unwrap();
    json!({"snapshot":snapshot,"result":{
        "format":"ASSET_OPTIMIZATION_DESKTOP_V2","sourceSha256":source.sha256,"coverage":batch.scope,
        "changes":{
            "sceneSplits":splits,
            "scenes":[{"episodeId":metadata_row.episode_id,"sceneId":format!("{}__L{}",metadata_row.scene_id.as_ref().unwrap(),metadata_row.number),"location":"合成测试平台"}],
            "assets":[{"name":"合成新增道具","type":"PROP","episodes":[row.episode_id],"sceneRefs":[[row.episode_id,child]],"preparation":{"mode":"REFERENCE"}}]
        }
    }})
}

pub(crate) fn final_editor() -> Value {
    let source = source();
    let prior = accepted_catalogue(&source);
    let first = editor();
    let first_input = &first["snapshot"]["input"];
    let first_batch = batch(
        &source,
        serde_json::from_value(first_input["scope"].clone()).unwrap(),
    );
    let mut raw = first["result"].clone();
    restore_delivery_refs(&mut raw, first_input).unwrap();
    let (prior, _) =
        apply_optimization(&raw, &source, &first_batch, first_input, &prior, None).unwrap();
    let scope: Vec<_> = source.episodes.iter().map(|e| e.id.clone()).collect();
    let projected = source_projection(&source, &prior.issues, &scope).unwrap();
    let batch = batch(
        &source,
        source.episodes[6..].iter().map(|e| e.id.clone()).collect(),
    );
    let snapshot = snapshot_for("Q1", &projected, &batch, &prior);
    let asset = &prior.assets[0];
    json!({"snapshot":snapshot,"result":{
        "format":"ASSET_OPTIMIZATION_DESKTOP_V2","sourceSha256":source.sha256,"coverage":batch.scope,
        "changes":{"assets":[{"key":asset.id,"variants":[{"key":asset.variants[0].id,"note":"合成基础形象说明修订","episodes":[],"sceneRefs":[],"scope":{"kind":"SCENES"}}]}]}
    }})
}

pub(crate) fn fixture(name: &str) -> Value {
    match name {
        "rejected-task.json" => catalogue(),
        "rejected-second-f2.json" => {
            let mut fixture = catalogue();
            fixture["result"]["notes"] = json!(["合成第二次返回，保留相同冻结请求"]);
            fixture
        }
        "rejected-q1.json" => editor(),
        "rejected-q1-final.json" => final_editor(),
        _ => panic!("unknown synthetic fixture: {name}"),
    }
}
