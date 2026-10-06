//! Stage2 design validity is deliberately independent of visual-fact receipts.
use super::{digest, resolve_asset_dependency, rule_snapshot};
use serde_json::{json, Value};
use std::collections::HashSet;

pub const DESIGN_PROFILE: &str = "asset-design-context/v2";
pub const PRESENTATION_PROFILE: &str = "asset-image-presentation/v1";
pub const CH_PRESENTATION: &str = "左侧大幅肖像/近景，右侧正面、侧面、背面全身；各区保持同一身份、发型、衣装和当前状态，全身头脚完整，肖像保留头顶发型，各分区清晰，不以浅景深虚化其它视图，干净中性背景，无资料栏、标签。";
fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap_or("")
}
fn first<'a>(values: impl IntoIterator<Item = &'a str>) -> &'a str {
    values
        .into_iter()
        .find(|s| !s.trim().is_empty())
        .unwrap_or("")
}
pub fn is_character(asset: &Value) -> bool {
    matches!(
        first([s(asset, "kind"), s(asset, "type")]),
        "CH" | "character" | "人物"
    )
}
pub fn identity(asset: &Value) -> &str {
    first([
        s(asset, "identityId"),
        s(asset, "baseAssetId"),
        s(asset, "id"),
    ])
}
pub fn project_visual_style(settings: &Value) -> &str {
    let style = first([
        s(settings, "projectVisualStyle"),
        s(settings, "visualStyle"),
    ])
    .trim();
    if style == "未选择" {
        ""
    } else {
        style
    }
}
pub fn production_direction(input: &Value) -> Value {
    if input["settings"]["productionDirectionConfirmed"] == true {
        input["settings"]["productionDirection"].clone()
    } else if input["foundation"]["directionConfirmed"] == true {
        input["foundation"]["productionDirection"].clone()
    } else {
        Value::Null
    }
}
/// Explicit per-asset composition wins; the project CH override never leaks to other kinds.
pub fn image_presentation(settings: &Value, asset: &Value) -> Value {
    let own = &asset["imagePresentation"];
    let project = &settings["imagePresentation"];
    let explicit = first([
        own.as_str().unwrap_or(""),
        s(own, "composition"),
        s(asset, "composition"),
    ]);
    let project_ch = first([project.as_str().unwrap_or(""), s(project, "composition")]);
    let composition = if !explicit.is_empty() {
        explicit
    } else if is_character(asset) {
        if project_ch.is_empty() {
            CH_PRESENTATION
        } else {
            project_ch
        }
    } else {
        ""
    };
    let ratio = first([
        s(own, "ratio"),
        s(asset, "ratio"),
        s(settings, "ratio"),
        if is_character(asset) { "16:9" } else { "" },
    ]);
    json!({"version":PRESENTATION_PROFILE,"composition":composition,"ratio":ratio,"origin":if !explicit.is_empty(){"asset"}else if is_character(asset)&&!project_ch.is_empty(){"project"}else{"default"}})
}
/// Never sanitize source words (for example a sign reading 第十集). Exact existing
/// components in legacy/manual prose are not duplicated, and saved prose is untouched.
pub fn compile_image_prompt(body: &str, style: &str, presentation: &Value) -> String {
    let body = body.trim();
    let style = style.trim();
    let layout = s(presentation, "composition").trim();
    let mut parts = Vec::new();
    if !style.is_empty() && !body.starts_with(style) {
        parts.push(style);
    }
    parts.push(body);
    if !layout.is_empty() && !body.contains(layout) && style != layout {
        parts.push(layout);
    }
    parts.join("\n\n")
}
/// A semantic version ignores save counters, adopted pictures and generated output.
pub fn asset_semantics(asset: &Value) -> Value {
    let mut result = json!({});
    for key in [
        "id",
        "identityId",
        "baseAssetId",
        "variantId",
        "name",
        "kind",
        "type",
        "description",
        "sourceRefs",
        "sourceRefsPurpose",
        "sourceScope",
        "scope",
        "applicability",
        "preparation",
        "imagePresentation",
        "composition",
        "ratio",
    ] {
        result[key] = asset[key].clone();
    }
    result
}
pub fn source_chain(assets: &[Value], asset: &Value) -> Result<Vec<Value>, String> {
    let mut chain = Vec::new();
    let mut current = asset;
    while let Some(id) = resolve_asset_dependency(assets, current)? {
        current = assets
            .iter()
            .find(|a| s(a, "id") == id)
            .ok_or("来源设计已不存在")?;
        chain.push(json!({"assetId":id,"identityId":identity(current),"semanticVersion":digest(&asset_semantics(current).to_string()),"designVersion":digest(&json!({"prompt":current["prompt"],"receipt":current["design"]["designReceipt"]}).to_string())}));
    }
    Ok(chain)
}
fn fact_semantics(fact: &Value) -> Value {
    json!({"id":fact["id"],"assetId":fact["assetId"],"variantId":fact["variantId"],"attribute":fact["attribute"],"value":fact["value"],"scope":fact["scope"],"claim":fact["claim"],"sourceRefs":fact["sourceRefs"]})
}
/// Resume may reuse the original workflow's published facts, never silently read
/// later edits into a request already bound to a design/source version.
pub fn validate_resume_facts(
    original: &Value,
    result: &Value,
    current: &Value,
) -> Result<(), String> {
    if original["source"]["sha256"] != current["source"]["sha256"] {
        return Err("图片续跑的锁定来源已变化".into());
    }
    if let Some(rule) = original["designRuleSha256"].as_str() {
        if rule_snapshot("asset-art-director")?["sha256"] != rule {
            return Err("图片续跑美术规则版本已变化；原请求保留，请发起新设计".into());
        }
    }
    let assets = current["assets"].as_array().ok_or("图片续跑缺少资产目录")?;
    let selection = current["selection"]["assetIds"].as_array();
    let mut subjects = HashSet::new();
    for asset in assets
        .iter()
        .filter(|a| selection.is_none_or(|ids| ids.contains(&a["id"])))
    {
        subjects.insert(identity(asset).to_owned());
        for source in source_chain(assets, asset)? {
            subjects.insert(s(&source, "identityId").to_owned());
        }
    }
    let expected = result
        .get("facts")
        .filter(|f| f.is_array())
        .unwrap_or(&original["foundation"]["facts"]);
    let normalized = |facts: &Value| {
        let mut facts: Vec<_> = facts
            .as_array()
            .into_iter()
            .flatten()
            .filter(|f| subjects.contains(s(f, "assetId")))
            .map(fact_semantics)
            .collect();
        facts.sort_by_key(Value::to_string);
        facts
    };
    if normalized(expected) != normalized(&current["foundation"]["facts"]) {
        return Err("图片续跑的相关视觉事实语义版本已变化；原包与预算保留，请发起新制作".into());
    }
    Ok(())
}
pub fn design_receipt(input: &Value, asset: &Value) -> Result<Value, String> {
    let assets = input["assets"].as_array().ok_or("缺少资产目录")?;
    let chain = source_chain(assets, asset)?;
    let subjects: HashSet<_> = std::iter::once(identity(asset))
        .chain(chain.iter().filter_map(|v| v["identityId"].as_str()))
        .collect();
    let mut facts: Vec<_> = input["foundation"]["facts"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| subjects.contains(s(f, "assetId")))
        .map(fact_semantics)
        .collect();
    facts.sort_by_key(Value::to_string);
    let basis = json!({"version":DESIGN_PROFILE,"ruleSha256":rule_snapshot("asset-art-director")?["sha256"],"sourceSha256":input["source"]["sha256"],"assetSemanticVersion":digest(&asset_semantics(asset).to_string()),"sourceChain":chain,"factsSha256":digest(&json!(facts).to_string()),"projectVisualStyle":project_visual_style(&input["settings"]),"productionDirection":production_direction(input),"imagePresentation":image_presentation(&input["settings"],asset)});
    Ok(json!({"version":DESIGN_PROFILE,"sha256":digest(&basis.to_string()),"basis":basis}))
}
pub fn design_receipt_current(input: &Value, asset: &Value) -> bool {
    design_receipt(input, asset).is_ok_and(|expected| asset["design"]["designReceipt"] == expected)
}
/// Bounded aesthetic context only: never a source fact, cache dependency or image reference.
pub fn comparison_designs(input: &Value, asset: &Value) -> Vec<Value> {
    if !is_character(asset) {
        return Vec::new();
    }
    let Some(assets) = input["assets"].as_array() else {
        return Vec::new();
    };
    let Ok(chain) = source_chain(assets, asset) else {
        return Vec::new();
    };
    let mut seen: HashSet<String> = std::iter::once(identity(asset).to_string())
        .chain(chain.iter().map(|a| s(a, "identityId").to_string()))
        .collect();
    let mut candidates: Vec<_> = assets
        .iter()
        .filter(|a| {
            is_character(a) && !crate::store::asset_deleted(a) && !s(a, "prompt").trim().is_empty()
        })
        .collect();
    candidates.sort_by_key(|a| (identity(a), s(a, "id")));
    let mut remaining: usize = 6400 - 2; // Include array brackets, separators and metadata in the cap.
    let mut result = Vec::new();
    for other in candidates {
        let ai = s(other, "promptOrigin") == "AI"
            || s(&other["design"], "schemaVersion") == "asset-design/v1";
        if ai
            && s(other, "promptOrigin") != "MANUAL"
            && super::assets::automatic_design_stale(input, other)
        {
            continue;
        }
        if !seen.insert(identity(other).to_string()) {
            continue;
        }
        let chars: Vec<_> = s(other, "prompt").chars().take(1600).collect();
        let mut item = json!({"assetId":other["id"],"identityId":identity(other),"prompt":"","usage":"AESTHETIC_COMPARISON_ONLY_NOT_SOURCE_OR_IMAGE_DEPENDENCY"});
        let separator = usize::from(!result.is_empty());
        let budget = remaining.saturating_sub(separator);
        let mut low = 0;
        let mut high = chars.len();
        while low < high {
            let mid = (low + high + 1) / 2;
            item["prompt"] = json!(chars[..mid].iter().collect::<String>());
            if item.to_string().chars().count() <= budget {
                low = mid;
            } else {
                high = mid - 1;
            }
        }
        if low == 0 {
            continue;
        }
        item["prompt"] = json!(chars[..low].iter().collect::<String>());
        remaining -= item.to_string().chars().count() + separator;
        result.push(item);
        if result.len() == 8 || remaining == 0 {
            break;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn basis() -> Value {
        json!({"source":{"sha256":"locked"},"foundation":{"facts":[]},"settings":{"projectVisualStyle":"水墨国漫","ratio":"9:16"},"assets":[{"id":"a","identityId":"a","variantId":"base","kind":"CH","prompt":"成熟深肤色人物","promptOrigin":"MANUAL"}]})
    }
    #[test]
    fn presentation_defaults_only_characters_and_preserves_explicit_ratio_and_layout() {
        for kind in ["CH", "character"] {
            for variant in ["base", "hurt"] {
                let asset = json!({"kind":kind,"variantId":variant});
                assert_eq!(
                    image_presentation(&json!({}), &asset)["composition"],
                    CH_PRESENTATION
                );
                assert_eq!(image_presentation(&json!({}), &asset)["ratio"], "16:9");
                assert_eq!(
                    image_presentation(&json!({"ratio":"9:16"}), &asset)["ratio"],
                    "9:16"
                );
            }
        }
        for kind in ["GROUP", "BEAST", "LOC", "PROP", "scene", "prop"] {
            assert_eq!(
                image_presentation(
                    &json!({"imagePresentation":"人物四视图"}),
                    &json!({"kind":kind})
                )["composition"],
                ""
            );
        }
        let explicit =
            json!({"kind":"CH","imagePresentation":{"composition":"单幅正面半身","ratio":"3:2"}});
        assert_eq!(
            image_presentation(
                &json!({"ratio":"16:9","imagePresentation":"项目构图"}),
                &explicit
            )["composition"],
            "单幅正面半身"
        );
        assert_eq!(
            image_presentation(&json!({"ratio":"16:9"}), &explicit)["ratio"],
            "3:2"
        );
    }
    #[test]
    fn compiler_preserves_visible_words_and_adds_components_once_without_rewriting_body() {
        let p = image_presentation(&json!({}), &json!({"kind":"CH"}));
        let body = "六十岁深肤色人物，招牌写“第十集”。";
        let prompt = compile_image_prompt(body, "水墨国漫", &p);
        assert_eq!(prompt.matches("水墨国漫").count(), 1);
        assert_eq!(prompt.matches(CH_PRESENTATION).count(), 1);
        assert!(prompt.contains(body));
        assert_eq!(compile_image_prompt(&prompt, "水墨国漫", &p), prompt);
    }
    #[test]
    fn design_cache_tracks_rule_style_presentation_asset_source_but_not_other_designs_or_fact_rule()
    {
        let mut input = basis();
        input["assets"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"b","kind":"CH","prompt":"另一人","promptOrigin":"MANUAL"}));
        let receipt = design_receipt(&input, &input["assets"][0]).unwrap();
        input["assets"][0]["design"] = json!({"designReceipt":receipt});
        input["assets"][1]["prompt"] = json!("另一人新设计");
        input["foundation"]["visualFactBatches"] = json!([{"ruleSha256":"changed"}]);
        input["assets"][0]["version"] = json!(88);
        assert!(design_receipt_current(&input, &input["assets"][0]));
        for (field, value) in [
            ("projectVisualStyle", json!("CG")),
            ("imagePresentation", json!("正面")),
            ("ratio", json!("3:2")),
            ("productionDirection", json!("朴素布料")),
        ] {
            let mut changed = input.clone();
            changed["settings"][field] = value;
            if field == "productionDirection" {
                changed["settings"]["productionDirectionConfirmed"] = json!(true);
            }
            assert!(
                !design_receipt_current(&changed, &changed["assets"][0]),
                "{field}"
            );
        }
        input["assets"][0]["description"] = json!("年龄与体态变更");
        assert!(!design_receipt_current(&input, &input["assets"][0]));
    }
    #[test]
    fn comparison_identity_exclusions_and_bounds_are_stable_and_not_dependencies() {
        let mut input = basis();
        let current = json!({"id":"child","identityId":"a","kind":"CH","sourceAssetId":"a"});
        input["assets"]
            .as_array_mut()
            .unwrap()
            .push(current.clone());
        for (id, kind, origin, extra) in [
            ("self-alias", "CH", "MANUAL", json!({"identityId":"a"})),
            ("deleted", "CH", "MANUAL", json!({"deletedAt":"now"})),
            ("beast", "BEAST", "MANUAL", json!({})),
            ("stale", "CH", "AI", json!({})),
        ] {
            let mut a = json!({"id":id,"kind":kind,"promptOrigin":origin,"prompt":"不该出现"});
            for (k, v) in extra.as_object().unwrap() {
                a[k] = v.clone();
            }
            input["assets"].as_array_mut().unwrap().push(a);
        }
        for n in 0..12 {
            input["assets"].as_array_mut().unwrap().push(json!({"id":format!("p{n:02}"),"identityId":format!("person{}",n/2),"kind":"CH","promptOrigin":"MANUAL","prompt":"样".repeat(2000)}));
        }
        let context = comparison_designs(&input, &current);
        assert_eq!(context.len(), 4);
        assert!(context
            .iter()
            .all(|a| s(a, "prompt").chars().count() <= 1600));
        assert!(json!(context).to_string().chars().count() <= 6400);
        assert!(context.iter().all(|a| s(a, "assetId").starts_with('p')));
        input["assets"].as_array_mut().unwrap().reverse();
        assert_eq!(comparison_designs(&input, &current), context);
        assert_eq!(
            source_chain(input["assets"].as_array().unwrap(), &current)
                .unwrap()
                .len(),
            1
        );
        for a in input["assets"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter(|a| s(a, "id").starts_with('p'))
        {
            a["prompt"] = json!("短设计");
            a["identityId"] = a["id"].clone();
        }
        assert_eq!(comparison_designs(&input, &current).len(), 8);
    }
    #[test]
    fn direct_source_design_semantics_invalidate_descendant_but_counters_do_not() {
        let mut input = basis();
        input["assets"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":"child","kind":"CH","sourceAssetId":"a"}));
        let receipt = design_receipt(&input, &input["assets"][1]).unwrap();
        input["assets"][1]["design"] = json!({"designReceipt":receipt});
        input["assets"][0]["version"] = json!(20);
        assert!(design_receipt_current(&input, &input["assets"][1]));
        input["assets"][0]["prompt"] = json!("原脸保持，新衣装");
        assert!(!design_receipt_current(&input, &input["assets"][1]));
    }
    #[test]
    fn resume_preserves_published_facts_but_rejects_relevant_later_edits_and_rule_change() {
        let original = basis();
        let fact = json!({"id":"f","assetId":"a","value":"原肤色"});
        let result = json!({"facts":[fact]});
        let mut current = original.clone();
        current["foundation"]["facts"] = json!([fact,{"assetId":"unrelated","value":"新衣装"}]);
        assert!(validate_resume_facts(&original, &result, &current).is_ok());
        current["foundation"]["facts"][0]["quotes"] = json!(["程序物化引文"]);
        assert!(validate_resume_facts(&original, &result, &current).is_ok());
        current["foundation"]["facts"][0]["value"] = json!("后来修改肤色");
        assert!(validate_resume_facts(&original, &result, &current).is_err());
        let mut old_rule = original.clone();
        old_rule["designRuleSha256"] = json!("old-art-rule");
        assert!(validate_resume_facts(&old_rule, &json!({}), &original).is_err());
    }
}
