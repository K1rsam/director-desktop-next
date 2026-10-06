use super::*;
use serde_json::json;

fn assert_original(source: &LockedSource, text: &str) {
    assert_eq!(source.text.as_bytes(), text.as_bytes());
    assert_eq!(source.sha256, digest(text));
    assert_eq!(source.length, text.encode_utf16().count());
    assert_eq!(
        source
            .rows
            .iter()
            .map(|r| r.text.as_str())
            .collect::<String>(),
        text
    );
    for row in &source.rows {
        assert_eq!(slice_utf16(text, &row.range).unwrap(), row.text);
        if let Some(q) = &row.dialogue {
            assert_eq!(slice_utf16(text, &q.range).unwrap(), q.quote);
            for part in q.spoken_segments.iter().chain(&q.performance_segments) {
                assert_eq!(slice_utf16(text, &part.range).unwrap(), part.text);
            }
        }
    }
    validate_source(source).unwrap();
}

#[test]
fn latin_suffix_names_and_combining_marks_keep_exact_identity_and_explicit_cues() {
    let text = "第一集\r\n场景：大厅\r\nCarlos：快走😀。\r\nBravo：继续。\r\nRíos：停下。\r\nRi\u{301}os：别动。\r\n小王os：知道。\r\nCarlos os：听见。\r\nBravo（v.o.）：回来。\r\nRíos [OS]：原话。\r\n系统VO：【消息😀】\r\n";
    let source = prepare_source(text).unwrap();
    assert_eq!(source.parser_version, FIDELITY_PARSER_VERSION);
    assert_eq!(
        source
            .rows
            .iter()
            .filter_map(|r| r.dialogue.as_ref())
            .map(|q| (q.speaker.as_str(), q.channel.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("Carlos", "现场"),
            ("Bravo", "现场"),
            ("Ríos", "现场"),
            ("Ri\u{301}os", "现场"),
            ("小王", "OS"),
            ("Carlos", "OS"),
            ("Bravo", "VO"),
            ("Ríos", "OS"),
            ("系统", "VO")
        ]
    );
    assert_eq!(
        spoken_text(source.rows.last().unwrap().dialogue.as_ref().unwrap()),
        "【消息😀】"
    );
    assert_original(&source, text);
}

#[test]
fn exact_inline_scene_dialogue_retains_scene_speaker_quote_and_utf16() {
    for line in [
        "1-1 日 内 大厅 甲：快走",
        "  1-1，日，内，大厅，Carlos（低声）：快😀走e\u{301}（抬手）。",
        "1-1、日、内、大厅、Ríos OS：快走",
    ] {
        let text = format!("第一集 😀e\u{301}\r\n{line}\r\n");
        let source = prepare_source(&text).unwrap();
        assert_eq!(source.parser_version, FIDELITY_PARSER_VERSION);
        assert_eq!(source.scenes.len(), 1, "{line}");
        let row = &source.rows[1];
        assert_eq!(row.kind, "body");
        assert_eq!(row.scene_id.as_ref(), Some(&source.scenes[0].id));
        let q = row.dialogue.as_ref().unwrap();
        assert_eq!(
            q.speaker,
            if line.contains("Carlos") {
                "Carlos"
            } else if line.contains("Ríos") {
                "Ríos"
            } else {
                "甲"
            }
        );
        let quote = line.split('：').next_back().unwrap();
        assert_eq!(q.quote, quote);
        assert_eq!(
            q.range.start,
            text[..text.find(quote).unwrap()].encode_utf16().count()
        );
        assert!(source.diagnostics.is_empty(), "{:?}", source.diagnostics);
        assert_original(&source, &text);
    }
}

#[test]
fn punctuation_scene_headers_and_numeric_narrative_are_distinct() {
    for separator in [",", "，", "、"] {
        let text = format!(
            "第一集\n1-1{separator}日{separator}内{separator}大厅\n甲：走。\n1.2 秒后 日 内 大厅\n"
        );
        let source = prepare_source(&text).unwrap();
        assert_eq!(source.scenes.len(), 1);
        assert_eq!(source.rows[1].kind, "format");
        assert_eq!(source.rows[3].kind, "body");
        assert_eq!(source.rows[3].scene_id, source.rows[1].scene_id);
        assert_original(&source, &text);
    }
    let source = prepare_source("第一集\n1.2 秒后 日 内 大厅\n").unwrap();
    assert!(source.scenes.is_empty());
    assert!(source
        .diagnostics
        .iter()
        .any(|d| d.code == "SCENE_BOUNDARY_UNCONFIRMED"));
}

#[test]
fn ambiguous_inline_scene_and_multiple_speakers_stay_blocking() {
    for text in [
        "第一集\n1-1 日 内 甲：快走\n",
        "第一集\n1-1 日 内 大厅 甲：快走；乙：等等\n",
    ] {
        let source = prepare_source(text).unwrap();
        assert_eq!(source.parser_version, FIDELITY_PARSER_VERSION);
        assert!(source.diagnostics.iter().any(|d| d.blocking
            && [
                "SCENE_PAYLOAD_UNCONFIRMED",
                "DIALOGUE_MULTI_SPEAKER_UNCONFIRMED"
            ]
            .contains(&d.code.as_str())));
        let mut forged = source.clone();
        forged.diagnostics.clear();
        assert!(validate_source(&forged).is_err());
        assert_original(&source, text);
    }
}

#[test]
fn old_v1_through_v5_replay_frozen_and_reject_new_metadata_without_reparse() {
    let text = "第一集\r\n场景：大厅\r\nCarlos：来😀。\r\n1-1 日 内 大厅 甲：快走\r\n△面板字幕：\r\n情报1：显示。\r\n1.2 秒后 日 内 大厅\r\n";
    for version in [
        LEGACY_PARSER_VERSION,
        CONTEXT_PARSER_VERSION,
        CURRENT_PARSER_VERSION,
        STRUCTURAL_PARSER_VERSION,
        DISPLAY_PARSER_VERSION,
    ] {
        let old = parse_source(text, version).unwrap();
        let bytes = serde_json::to_vec(&old).unwrap();
        assert_eq!(
            serde_json::to_vec(&upgrade_source_dialogues(&old).unwrap()).unwrap(),
            bytes
        );
        validate_source(&old).unwrap();
        let recognized = reparse_source(&old).unwrap();
        assert_eq!(recognized.parser_version, FIDELITY_PARSER_VERSION);
        assert_original(&recognized, text);
        assert_eq!(
            old.rows
                .iter()
                .map(|r| (&r.id, &r.range, &r.text))
                .collect::<Vec<_>>(),
            recognized
                .rows
                .iter()
                .map(|r| (&r.id, &r.range, &r.text))
                .collect::<Vec<_>>()
        );
        let mut counterfeit = old.clone();
        counterfeit.rows[2].dialogue = recognized.rows[2].dialogue.clone();
        if version != LEGACY_PARSER_VERSION && version != CONTEXT_PARSER_VERSION {
            assert!(validate_source(&counterfeit).is_err());
        }
        let mut damaged = recognized.clone();
        damaged.rows[3].dialogue.as_mut().unwrap().range.start += 1;
        assert!(validate_source(&damaged).is_err());
        assert_eq!(serde_json::to_vec(&old).unwrap(), bytes);
    }
    assert_eq!(
        prepare_source("第一集\n场景：大厅\n甲：走。\n")
            .unwrap()
            .parser_version,
        CURRENT_PARSER_VERSION
    );
    assert_eq!(
        prepare_source("第1集\n第一集\n场景：大厅\n甲：走。\n")
            .unwrap()
            .parser_version,
        STRUCTURAL_PARSER_VERSION
    );
    assert_eq!(
        prepare_source("第一集\n场景：大厅\n△面板字幕：\n情报1：显示。\n")
            .unwrap()
            .parser_version,
        DISPLAY_PARSER_VERSION
    );
}

fn body_decision(source: &LockedSource, row: usize, number: u32) -> BoundaryDecision {
    BoundaryDecision {
        start: source.rows[row].range.start,
        end: source.rows[row].range.end,
        kind: "body".into(),
        episode_number: Some(number),
        scene_title: Some(format!("场{row}")),
        reason: "用户明确确认".into(),
        decision_version: None,
        confirmation: None,
    }
}

#[test]
fn decisions_later_then_earlier_rebuild_episode_scene_and_member_order() {
    let source = prepare_source("甲：先到😀。\n乙：后来。\n").unwrap();
    let later = apply_boundary_decisions(&source, &[body_decision(&source, 1, 2)]).unwrap();
    let both = apply_boundary_decisions(&later, &[body_decision(&later, 0, 1)]).unwrap();
    assert_eq!(
        both.episodes
            .iter()
            .map(|e| e.source_number.as_str())
            .collect::<Vec<_>>(),
        ["1", "2"]
    );
    assert_eq!(
        both.episodes.iter().map(|e| e.number).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(
        both.scenes
            .iter()
            .map(|s| s.row_ids[0].as_str())
            .collect::<Vec<_>>(),
        both.rows.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()
    );
    let same = apply_boundary_decisions(&source, &[body_decision(&source, 1, 1)]).unwrap();
    let same = apply_boundary_decisions(&same, &[body_decision(&same, 0, 1)]).unwrap();
    assert_eq!(same.episodes.len(), 1);
    assert_eq!(
        same.episodes[0].scene_ids,
        same.scenes.iter().map(|s| s.id.clone()).collect::<Vec<_>>()
    );
    assert_eq!(same.scenes[0].row_ids, [source.rows[0].id.clone()]);
    assert_original(&same, &source.text);
    let reopened: LockedSource =
        serde_json::from_slice(&serde_json::to_vec(&both).unwrap()).unwrap();
    validate_source(&reopened).unwrap();
    assert_eq!(
        serde_json::to_value(&reopened).unwrap(),
        serde_json::to_value(upgrade_source_dialogues(&both).unwrap()).unwrap()
    );
}

fn duplicate_decision(source: &LockedSource, row: usize) -> BoundaryDecision {
    BoundaryDecision {
        start: source.rows[row].range.start,
        end: source.rows[row].range.end,
        kind: "independent_episode".into(),
        episode_number: Some(1),
        scene_title: None,
        reason: "保留为独立制作段".into(),
        decision_version: None,
        confirmation: Some(BoundaryConfirmation {
            source_version: 7,
            source_sha256: source.sha256.clone(),
            parser_version: source.parser_version.clone(),
            heading_row_id: source.rows[row].id.clone(),
            diagnostic_code: "DUPLICATE_EPISODE_NUMBER".into(),
        }),
    }
}

#[test]
fn duplicate_episode_confirmation_is_exact_persistent_and_cannot_be_borrowed() {
    let text = "第一集\n场景：前厅\n甲：先到。\n第一集\n场景：后厅\n甲：后来。\n第一集\n场景：门口\n甲：再来。\n";
    let source = prepare_source(text).unwrap();
    let decision = duplicate_decision(&source, 3);
    let one = apply_boundary_decisions(&source, std::slice::from_ref(&decision)).unwrap();
    assert_eq!(
        one.diagnostics
            .iter()
            .filter(|d| d.code == "DUPLICATE_EPISODE_NUMBER")
            .count(),
        1
    );
    assert_eq!(one.episodes.len(), 3);
    let repeated = apply_boundary_decisions(&one, std::slice::from_ref(&decision)).unwrap();
    assert_eq!(
        serde_json::to_value(&one).unwrap(),
        serde_json::to_value(repeated).unwrap()
    );
    let mut refreshed = duplicate_decision(&one, 3);
    refreshed.confirmation.as_mut().unwrap().source_version = 8;
    assert_eq!(
        serde_json::to_value(&one).unwrap(),
        serde_json::to_value(apply_boundary_decisions(&one, &[refreshed]).unwrap()).unwrap()
    );
    let mut borrowed = duplicate_decision(&source, 6);
    borrowed.confirmation = decision.confirmation.clone();
    assert!(apply_boundary_decisions(&source, &[borrowed]).is_err());
    for mutate in 0..5 {
        let mut forged = decision.clone();
        match mutate {
            0 => forged.confirmation.as_mut().unwrap().source_sha256 = "0".repeat(64),
            1 => forged.end = source.length,
            2 => forged.confirmation.as_mut().unwrap().source_version = 0,
            3 => {
                forged.confirmation.as_mut().unwrap().diagnostic_code =
                    "BOUNDARY_UNCONFIRMED".into()
            }
            _ => {
                forged.confirmation.as_mut().unwrap().parser_version = LEGACY_PARSER_VERSION.into()
            }
        }
        assert!(apply_boundary_decisions(&source, &[forged]).is_err());
    }
    let mut broad = body_decision(&source, 3, 1);
    broad.end = source.length;
    assert!(apply_boundary_decisions(&source, &[broad]).is_err());
    let both = apply_boundary_decisions(&one, &[duplicate_decision(&one, 6)]).unwrap();
    assert!(both.diagnostics.is_empty());
    let saved: LockedSource = serde_json::from_slice(&serde_json::to_vec(&both).unwrap()).unwrap();
    assert_original(&saved, text);
    assert_eq!(
        saved.episodes.iter().map(|e| &e.id).collect::<Vec<_>>(),
        source.episodes.iter().map(|e| &e.id).collect::<Vec<_>>()
    );
}

#[test]
fn old_unsorted_decision_dialect_stays_valid_without_silent_reordering() {
    let source = parse_source("甲：先到。\n乙：后来。\n", CURRENT_PARSER_VERSION).unwrap();
    let old = apply_decisions_inner(
        &source,
        &[body_decision(&source, 1, 2), body_decision(&source, 0, 1)],
    )
    .unwrap();
    assert_eq!(old.episodes[0].source_number, "2");
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(upgrade_source_dialogues(&old).unwrap()).unwrap()
    );
}

fn split_and_metadata(source: LockedSource, row_index: usize) -> LockedSource {
    let sid = source.scenes[0].id.clone();
    let eid = source.scenes[0].episode_id.clone();
    let row = &source.rows[row_index];
    let edits = vec![
        json!({"kind":"sceneSplit","sourceSha256":source.sha256,"plan":{"episodeId":eid,"sceneId":sid,"cuts":[{"rowId":row.id,"anchor":row.text.trim_end_matches(['\r','\n'])}]}}),
        json!({"kind":"sceneMetadata","sourceSha256":source.sha256,"plan":{"episodeId":eid,"sceneId":format!("{sid}__L{}",row.number),"sourceRefs":[row.id],"location":"已确认后厅"}}),
    ];
    let out = super::super::scene_edits::replay_scene_edits(source, &edits).unwrap();
    validate_source(&out).unwrap();
    out
}

#[test]
fn reparse_remaps_split_and_metadata_only_with_exact_scene_and_episode_evidence() {
    let text = "第1集\r\n第一集\r\n场景：大厅\r\n甲：先到。\r\n乙：后来😀。\r\n";
    let old = split_and_metadata(parse_source(text, CURRENT_PARSER_VERSION).unwrap(), 4);
    let bytes = serde_json::to_vec(&old).unwrap();
    let recognized = reparse_source(&old).unwrap();
    assert_eq!(recognized.episodes.len(), 1);
    assert_ne!(
        old.scene_edits[0]["plan"]["episodeId"],
        recognized.scene_edits[0]["plan"]["episodeId"]
    );
    assert_eq!(recognized.scenes[1].metadata["location"], "已确认后厅");
    assert_eq!(
        old.scenes.iter().map(|s| &s.row_ids).collect::<Vec<_>>(),
        recognized
            .scenes
            .iter()
            .map(|s| &s.row_ids)
            .collect::<Vec<_>>()
    );
    assert_original(&recognized, text);
    assert_eq!(serde_json::to_vec(&old).unwrap(), bytes);
    assert_eq!(
        serde_json::to_value(&recognized).unwrap(),
        serde_json::to_value(reparse_source(&recognized).unwrap()).unwrap()
    );
}

#[test]
fn reparse_refuses_same_hash_when_scene_members_changed_and_preserves_old_edits() {
    let text = "第一集\n场景：大厅\n甲：先到。\n1-2日 内 后厅\n乙：后来。\n";
    let old = split_and_metadata(parse_source(text, CURRENT_PARSER_VERSION).unwrap(), 4);
    let saved = serde_json::to_vec(&old).unwrap();
    assert!(reparse_source(&old).unwrap_err().contains("无法唯一证明"));
    assert_eq!(serde_json::to_vec(&old).unwrap(), saved);
    validate_source(&old).unwrap();
}

#[test]
fn unrelated_location_first_dashed_scene_keeps_old_contract() {
    let text = "第一集\n1-1 大厅 日 内\n甲：走。\n";
    let source = prepare_source(text).unwrap();
    assert_eq!(source.parser_version, CURRENT_PARSER_VERSION);
    assert_eq!(source.scenes.len(), 1);
    assert_original(&source, text);
}

#[test]
fn reparse_rejects_boundary_decision_when_episode_membership_cannot_be_proven() {
    let source = parse_source("第一集\n第一集\n甲：走。\n", CURRENT_PARSER_VERSION).unwrap();
    let old = apply_decisions_inner(&source, &[body_decision(&source, 0, 1)]).unwrap();
    let saved = serde_json::to_vec(&old).unwrap();
    assert!(reparse_source(&old).unwrap_err().contains("集成员"));
    assert_eq!(serde_json::to_vec(&old).unwrap(), saved);
    validate_source(&old).unwrap();
}
