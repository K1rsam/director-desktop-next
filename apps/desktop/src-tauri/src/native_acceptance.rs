//! Bounded native CLI acceptance. This is an operator-loaded scope, never a webview toggle.
//! Source, selected episodes, backend/model, and each workflow reservation survive restarts.
use crate::{
    pipeline::{digest, validate_source, LockedSource},
    store::{s, Result},
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

pub const VERSION: &str = "native-cli-acceptance/v1";
pub const MODEL: &str = "gpt-6.1-sol";
pub const REASONING: &str = "medium";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AcceptanceGrant {
    pub version: String,
    pub grant_id: String,
    pub approved_document_sha256: String,
    pub locked_source_sha256: String,
    pub episode_count: usize,
    pub storyboard_episode_numbers: Vec<u32>,
    pub backend: String,
    pub model: String,
    pub reasoning: String,
}
fn sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
impl AcceptanceGrant {
    pub fn validate(&self) -> Result<()> {
        if self.version != VERSION
            || self.grant_id.is_empty()
            || self.grant_id.len() > 128
            || !self
                .grant_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        {
            return Err("验收授权标识或版本无效".into());
        }
        if !sha(&self.approved_document_sha256)
            || !sha(&self.locked_source_sha256)
            || self.episode_count != 10
        {
            return Err("验收必须绑定获准的完整10集源文件和提取文本哈希".into());
        }
        if self.storyboard_episode_numbers != vec![1, 2]
            || self.backend != "codex"
            || self.model != MODEL
            || self.reasoning != REASONING
        {
            return Err("本轮仅 CLI / gpt-6.1-sol / medium，第三阶段只第1、2集；不静默替换".into());
        }
        Ok(())
    }
    pub fn from_file(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            return Err("验收授权文件须为显式绝对路径".into());
        }
        let bytes = fs::read(path).map_err(|e| e.to_string())?;
        if bytes.len() > 16_384 {
            return Err("验收授权文件过大".into());
        }
        let grant: Self =
            serde_json::from_slice(&bytes).map_err(|_| "验收授权文件结构无效".to_string())?;
        grant.validate()?;
        Ok(grant)
    }
    pub fn verify_document(&self, bytes: &[u8]) -> Result<()> {
        use sha2::{Digest, Sha256};
        if format!("{:x}", Sha256::digest(bytes)) != self.approved_document_sha256 {
            return Err("验收源文件字节不符，未导入或执行".into());
        }
        Ok(())
    }
    pub fn verify_source(&self, source: &Value) -> Result<LockedSource> {
        self.validate()?;
        let locked: LockedSource =
            serde_json::from_value(source.clone()).map_err(|e| e.to_string())?;
        validate_source(&locked)?;
        if locked.diagnostics.iter().any(|d| d.blocking) {
            return Err("验收源仍有未决集场边界，未启动模型".into());
        }
        if locked.sha256 != self.locked_source_sha256 || locked.episodes.len() != self.episode_count
        {
            return Err("当前锁定原文不属于获准验收源，禁止真实执行".into());
        }
        let chinese = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "十"];
        for episode in &locked.episodes {
            let source_number = episode.source_number.parse::<u32>().ok().or_else(|| {
                chinese
                    .iter()
                    .position(|n| *n == episode.source_number)
                    .map(|n| n as u32 + 1)
            });
            if source_number != Some(episode.number) {
                return Err("原文集号不能与获准1至10集唯一对应".into());
            }
        }
        let numbers: BTreeSet<_> = locked.episodes.iter().map(|e| e.number).collect();
        if numbers != (1..=10).collect() {
            return Err("验收集序号不完整或有歧义".into());
        }
        Ok(locked)
    }
    pub fn verify_settings(&self, settings: &Value) -> Result<()> {
        if s(settings, "textProvider") != self.backend
            || s(settings, "textModel") != self.model
            || s(settings, "reasoning") != self.reasoning
        {
            return Err("验收设置与已批准 CLI/model/reasoning 不符，未自动改写或降档".into());
        }
        Ok(())
    }
    /// Returns the single permitted phase; source/setting changes never acquire a fresh budget.
    pub fn check_start(
        &self,
        request: &Value,
        source: &Value,
        settings: &Value,
        creative_ready: bool,
    ) -> Result<&'static str> {
        let locked = self.verify_source(source)?;
        self.verify_settings(settings)?;
        match s(request, "action") {
            "foundation.run" => {
                if let Some(ids) = request.get("episodeIds") {
                    let actual: BTreeSet<_> = ids
                        .as_array()
                        .ok_or("制作集须为数组")?
                        .iter()
                        .map(|id| id.as_str().ok_or_else(|| "制作集ID无效".to_string()))
                        .collect::<Result<_>>()?;
                    let expected: BTreeSet<_> =
                        locked.episodes.iter().map(|e| e.id.as_str()).collect();
                    if actual != expected || ids.as_array().unwrap().len() != expected.len() {
                        return Err("第一阶段验收固定完整10集范围".into());
                    }
                }
                Ok("foundation")
            }
            "images.design" => Ok("asset_design"),
            "storyboard.run" => {
                if !creative_ready {
                    return Err("STAGE3_ADAPTER_NOT_VERIFIED: 原件来源一致不等于适配已通过".into());
                }
                if request.get("unitIds").is_some() || request.get("revisionInstruction").is_some()
                {
                    return Err("本轮首发验收不能借局部重做扩大或重置预算".into());
                }
                let ids = request["episodeIds"]
                    .as_array()
                    .ok_or("分镜验收必须明确选择第1、2集")?;
                let actual: BTreeSet<_> = ids
                    .iter()
                    .map(|id| id.as_str().ok_or_else(|| "分镜集ID无效".to_string()))
                    .collect::<Result<_>>()?;
                let expected: BTreeSet<_> = locked
                    .episodes
                    .iter()
                    .filter(|e| self.storyboard_episode_numbers.contains(&e.number))
                    .map(|e| e.id.as_str())
                    .collect();
                if actual != expected || ids.len() != 2 {
                    return Err("第三阶段验收仅允许精确第1、2集，不能省略范围或包含其他集".into());
                }
                Ok("storyboard")
            }
            _ => Err("NATIVE_ACCEPTANCE_SCOPE_BLOCKED: 此动作不在有界文本验收首发范围".into()),
        }
    }
}

#[derive(Clone)]
pub struct NativeAcceptance {
    pub grant: AcceptanceGrant,
    database: PathBuf,
    _owner: Arc<Mutex<rusqlite::Connection>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reservation {
    pub phase: String,
    pub request_sha256: String,
    pub project_id: String,
}
impl NativeAcceptance {
    pub fn open(home: &Path, grant: AcceptanceGrant) -> Result<Self> {
        if !cfg!(any(target_os = "macos", target_os = "windows")) {
            return Err("原生验收模式仅在实际 Mac/Windows 启动；云端 Linux 保持离线".into());
        }
        Self::open_store(home, grant)
    }
    fn open_store(home: &Path, grant: AcceptanceGrant) -> Result<Self> {
        grant.validate()?;
        crate::platform::ensure_isolated_home(home)?;
        let owner = rusqlite::Connection::open(home.join("native-acceptance-owner.sqlite"))
            .map_err(|e| e.to_string())?;
        owner
            .busy_timeout(std::time::Duration::ZERO)
            .map_err(|e| e.to_string())?;
        owner.execute_batch("BEGIN IMMEDIATE").map_err(|_| {
            "NATIVE_ACCEPTANCE_OWNER_BUSY: 已有原生验收进程持有此目录；不能并发接管".to_string()
        })?;
        let database = home.join("native-acceptance.sqlite");
        let result = Self {
            grant,
            database,
            _owner: Arc::new(Mutex::new(owner)),
        };
        let c = result.conn()?;
        c.execute_batch("CREATE TABLE IF NOT EXISTS acceptance_grants(id TEXT PRIMARY KEY,sha TEXT NOT NULL,data TEXT NOT NULL); CREATE TABLE IF NOT EXISTS acceptance_runs(grant_id TEXT NOT NULL,phase TEXT NOT NULL,project_id TEXT NOT NULL,request_sha TEXT NOT NULL,request TEXT NOT NULL,source_sha TEXT NOT NULL,workflow_id TEXT,snapshot_sha TEXT,state TEXT NOT NULL,PRIMARY KEY(grant_id,phase)); CREATE UNIQUE INDEX IF NOT EXISTS acceptance_source_phase ON acceptance_runs(source_sha,phase); CREATE TABLE IF NOT EXISTS acceptance_events(id INTEGER PRIMARY KEY,grant_id TEXT,phase TEXT,data TEXT);").map_err(|e|e.to_string())?;
        let data = serde_json::to_string(&result.grant).map_err(|e| e.to_string())?;
        let prior: Option<String> = c
            .query_row(
                "SELECT sha FROM acceptance_grants WHERE id=?1",
                [&result.grant.grant_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        match prior {
            Some(old) if old != digest(&data) => {
                return Err("同一验收授权不能修改源、模型或范围以重置预算".into())
            }
            Some(_) => {}
            None => {
                c.execute(
                    "INSERT INTO acceptance_grants VALUES(?1,?2,?3)",
                    params![result.grant.grant_id, digest(&data), data],
                )
                .map_err(|e| e.to_string())?;
            }
        }
        Ok(result)
    }
    fn conn(&self) -> Result<rusqlite::Connection> {
        let c = rusqlite::Connection::open(&self.database).map_err(|e| e.to_string())?;
        c.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| e.to_string())?;
        Ok(c)
    }
    pub fn reserve(
        &self,
        request: &Value,
        source: &Value,
        settings: &Value,
        creative_ready: bool,
    ) -> Result<Reservation> {
        let phase = self
            .grant
            .check_start(request, source, settings, creative_ready)?;
        let project_id = s(request, "projectId");
        if project_id.is_empty() {
            return Err("验收缺少独立项目身份".into());
        }
        let request_data = json!({"request":request,"settings":settings,"sourceSha256":source["sha256"],"sourceVersion":source["version"]});
        let request_sha = digest(&request_data.to_string());
        let mut c = self.conn()?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let prior:Option<(String,String,String,String)>=tx.query_row("SELECT grant_id,project_id,state,request_sha FROM acceptance_runs WHERE source_sha=?1 AND phase=?2",params![self.grant.locked_source_sha256,phase],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?;
        match prior {
            Some((grant, project, state, old_hash))
                if grant == self.grant.grant_id
                    && project == project_id
                    && state == "not_started" =>
            {
                tx.execute("INSERT INTO acceptance_events(grant_id,phase,data) VALUES(?1,?2,?3)",params![self.grant.grant_id,phase,json!({"event":"retry_proven_unstarted","previousRequestSha256":old_hash,"nextRequestSha256":request_sha}).to_string()]).map_err(|e|e.to_string())?;
                tx.execute("UPDATE acceptance_runs SET request_sha=?1,request=?2,state='reserved' WHERE grant_id=?3 AND phase=?4 AND state='not_started'",params![request_sha,request_data.to_string(),self.grant.grant_id,phase]).map_err(|e|e.to_string())?;
            }
            Some(_) => return Err(
                "NATIVE_ACCEPTANCE_ALREADY_RESERVED: 保留原工作流；不能重开或换项目重置验收预算"
                    .into(),
            ),
            None => {
                tx.execute("INSERT INTO acceptance_runs(grant_id,phase,project_id,request_sha,request,source_sha,state) VALUES(?1,?2,?3,?4,?5,?6,'reserved')",params![self.grant.grant_id,phase,project_id,request_sha,request_data.to_string(),self.grant.locked_source_sha256]).map_err(|e|e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(Reservation {
            phase: phase.into(),
            request_sha256: request_sha,
            project_id: project_id.into(),
        })
    }
    pub fn receipt(&self, reservation: &Reservation) -> Value {
        json!({"version":VERSION,"grantId":self.grant.grant_id,"phase":reservation.phase,
            "projectId":reservation.project_id,"requestSha256":reservation.request_sha256,
            "sourceSha256":self.grant.locked_source_sha256})
    }
    pub fn bind_workflow(&self, reservation: &Reservation, job: &Value) -> Result<()> {
        if s(job, "id").is_empty() || s(job, "kind") != "workflow" {
            return Err("验收只能绑定实际持久工作流".into());
        }
        if job["snapshot"]["nativeAcceptance"] != self.receipt(reservation) {
            return Err("验收工作流缺少程序冻结的原预约标记".into());
        }
        let settings = job["snapshot"]
            .get("settings")
            .unwrap_or(&job["snapshot"]["input"]["settings"]);
        self.grant.verify_settings(settings)?;
        let c = self.conn()?;
        let n=c.execute("UPDATE acceptance_runs SET workflow_id=?1,snapshot_sha=?2,state='bound' WHERE grant_id=?3 AND phase=?4 AND project_id=?5 AND request_sha=?6 AND state='reserved' AND workflow_id IS NULL",params![s(job,"id"),digest(&job["snapshot"].to_string()),self.grant.grant_id,reservation.phase,reservation.project_id,reservation.request_sha256]).map_err(|e|e.to_string())?;
        if n != 1 {
            return Err("验收工作流已绑定或预约变化，未覆盖原任务".into());
        }
        Ok(())
    }
    /// Called before this process starts any worker, while its SQLite ownership lock
    /// excludes another native process. Unbound means execution has not crossed bind.
    pub fn reconcile_unbound(&self, store: &crate::store::Store) -> Result<()> {
        let project = store.project()?;
        let project_id = s(&project, "id");
        let mut ledger = self.conn()?;
        let mut q=ledger.prepare("SELECT phase,request_sha FROM acceptance_runs WHERE grant_id=?1 AND project_id=?2 AND state='reserved' AND workflow_id IS NULL").map_err(|e|e.to_string())?;
        let rows = q
            .query_map(params![self.grant.grant_id, project_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        drop(q);
        for (phase, request_sha) in rows {
            let c = store.conn()?;
            let mut jobs=c.prepare("SELECT id FROM jobs WHERE kind='workflow' AND json_extract(snapshot,'$.nativeAcceptance.grantId')=?1 AND json_extract(snapshot,'$.nativeAcceptance.phase')=?2").map_err(|e|e.to_string())?;
            let ids = jobs
                .query_map(params![self.grant.grant_id, phase], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(|e| e.to_string())?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            drop(jobs);
            let reservation = Reservation {
                phase: phase.clone(),
                request_sha256: request_sha.clone(),
                project_id: project_id.into(),
            };
            if ids.len() == 1 {
                self.bind_workflow(&reservation, &store.job(&ids[0])?)?;
                continue;
            }
            if !ids.is_empty() {
                return Err(
                    "NATIVE_ACCEPTANCE_AMBIGUOUS: 多个原工作流声称同一预约，保留所有权且不重发"
                        .into(),
                );
            }
            let pattern = match phase.as_str() {
                "foundation" => "foundation%",
                "asset_design" => "asset%",
                _ => "storyboard%",
            };
            let has_text:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE kind='text' AND json_extract(snapshot,'$.stage') LIKE ?1)",[pattern],|r|r.get(0)).map_err(|e|e.to_string())?;
            if has_text {
                return Err(
                    "NATIVE_ACCEPTANCE_AMBIGUOUS: 存在无父流程的同阶段文本证据，不能认定未派发"
                        .into(),
                );
            }
            let tx = ledger
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|e| e.to_string())?;
            let n=tx.execute("UPDATE acceptance_runs SET state='not_started' WHERE grant_id=?1 AND phase=?2 AND project_id=?3 AND request_sha=?4 AND state='reserved' AND workflow_id IS NULL",params![self.grant.grant_id,phase,project_id,request_sha]).map_err(|e|e.to_string())?;
            if n != 1 {
                return Err("NATIVE_ACCEPTANCE_RECONCILIATION_RACE: 保留原预约".into());
            }
            tx.execute("INSERT INTO acceptance_events(grant_id,phase,data) VALUES(?1,?2,?3)",params![self.grant.grant_id,phase,json!({"event":"orphan_proven_unstarted","requestSha256":request_sha,"modelDispatchPossible":false,"proof":"exclusive-owner/no-workflow/no-phase-text"}).to_string()]).map_err(|e|e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    pub fn check_resume(&self, request: &Value, job: &Value, source: &Value) -> Result<()> {
        self.grant.verify_source(source)?;
        if !["workflows.resume", "storyboard.decision.resolve"].contains(&s(request, "action"))
            || request["workflowId"] != job["id"]
        {
            return Err("验收续跑必须指向原工作流".into());
        }
        let c = self.conn()?;
        let receipt = &job["snapshot"]["nativeAcceptance"];
        if receipt["grantId"] == self.grant.grant_id {
            let pending:bool=c.query_row("SELECT EXISTS(SELECT 1 FROM acceptance_runs WHERE grant_id=?1 AND phase=?2 AND project_id=?3 AND request_sha=?4 AND state='reserved' AND workflow_id IS NULL)",params![self.grant.grant_id,s(receipt,"phase"),s(request,"projectId"),s(receipt,"requestSha256")],|r|r.get(0)).map_err(|e|e.to_string())?;
            if pending {
                let reservation: Reservation =
                    serde_json::from_value(receipt.clone()).map_err(|_| "原验收预约标记无效")?;
                self.bind_workflow(&reservation, job)?;
            }
        }
        let saved:Option<(String,String)>=c.query_row("SELECT project_id,snapshot_sha FROM acceptance_runs WHERE grant_id=?1 AND workflow_id=?2 AND state='bound'",params![self.grant.grant_id,s(job,"id")],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e|e.to_string())?;
        if !saved.is_some_and(|(project, hash)| {
            project == s(request, "projectId") && hash == digest(&job["snapshot"].to_string())
        }) {
            return Err("原验收任务的项目或冻结快照不符，禁止新派发".into());
        }
        Ok(())
    }
    pub fn status(&self, creative_ready: bool) -> Value {
        json!({"version":VERSION,"grantId":self.grant.grant_id,"sourceSha256":self.grant.locked_source_sha256,
            "backend":"codex","model":MODEL,"reasoning":REASONING,"storyboardEpisodeNumbers":[1,2],
            "stage3AdapterVerified":creative_ready,"sdkLiveAllowed":false,"realVideoAllowed":false,
            "imageSamples":"blocked_until_explicit_verified_free_case","newWorkflowsPerPhase":1})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (AcceptanceGrant, Value, Value) {
        let text = (1..=10)
            .map(|n| format!("第{n}集\n场景：合成地点\n甲：第{n}次测试🙂。\n"))
            .collect::<String>();
        let source = serde_json::to_value(crate::pipeline::prepare_source(&text).unwrap()).unwrap();
        let grant = AcceptanceGrant {
            version: VERSION.into(),
            grant_id: "synthetic-only".into(),
            approved_document_sha256: "a".repeat(64),
            locked_source_sha256: digest(&text),
            episode_count: 10,
            storyboard_episode_numbers: vec![1, 2],
            backend: "codex".into(),
            model: MODEL.into(),
            reasoning: REASONING.into(),
        };
        (
            grant,
            source,
            json!({"textProvider":"codex","textModel":MODEL,"reasoning":REASONING}),
        )
    }
    #[test]
    fn exact_source_model_and_two_episode_scope_are_required() {
        let (g, source, settings) = fixture();
        let ids = source["episodes"]
            .as_array()
            .unwrap()
            .iter()
            .take(2)
            .map(|e| e["id"].clone())
            .collect::<Vec<_>>();
        let request = json!({"action":"storyboard.run","projectId":"p","episodeIds":ids});
        assert!(g.check_start(&request, &source, &settings, true).is_ok());
        assert!(g.check_start(&request, &source, &settings, false).is_err());
        let mut wider = request.clone();
        wider["episodeIds"]
            .as_array_mut()
            .unwrap()
            .push(source["episodes"][2]["id"].clone());
        assert!(g.check_start(&wider, &source, &settings, true).is_err());
        for field in ["textProvider", "textModel", "reasoning"] {
            let mut wrong = settings.clone();
            wrong[field] = json!("different");
            assert!(g.check_start(&request, &source, &wrong, true).is_err());
        }
        let mut edited = source.clone();
        edited["text"] = json!("改写");
        assert!(g.check_start(&request, &edited, &settings, true).is_err());
        for action in [
            "images.run",
            "images.produce",
            "videos.submit",
            "units.regenerate",
            "foundation.choices.apply",
        ] {
            assert!(g
                .check_start(&json!({"action":action}), &source, &settings, true)
                .is_err());
        }
    }
    #[test]
    fn reservation_survives_restart_and_new_project_cannot_reset_it() {
        let (g, source, settings) = fixture();
        let dir = tempfile::tempdir().unwrap();
        let session = NativeAcceptance::open_store(dir.path(), g.clone()).unwrap();
        let request = json!({"action":"foundation.run","projectId":"p"});
        let reservation = session
            .reserve(&request, &source, &settings, false)
            .unwrap();
        let job = json!({"id":"original-workflow","kind":"workflow","snapshot":{"sourceSha256":source["sha256"],"request":request,"settings":settings,"nativeAcceptance":session.receipt(&reservation)}});
        session.bind_workflow(&reservation, &job).unwrap();
        drop(session);
        let reopened = NativeAcceptance::open_store(dir.path(), g).unwrap();
        assert!(reopened
            .reserve(&request, &source, &settings, false)
            .is_err());
        assert!(reopened
            .reserve(
                &json!({"action":"foundation.run","projectId":"different"}),
                &source,
                &settings,
                false
            )
            .is_err());
        let resume =
            json!({"action":"workflows.resume","projectId":"p","workflowId":"original-workflow"});
        assert!(reopened.check_resume(&resume, &job, &source).is_ok());
        let mut changed = job.clone();
        changed["snapshot"]["request"]["extra"] = json!(true);
        assert!(reopened.check_resume(&resume, &changed, &source).is_err());
    }
    #[test]
    fn copied_grant_identifier_cannot_change_source_or_model() {
        let (mut grant, _, _) = fixture();
        let dir = tempfile::tempdir().unwrap();
        NativeAcceptance::open_store(dir.path(), grant.clone()).unwrap();
        grant.locked_source_sha256 = "b".repeat(64);
        assert!(NativeAcceptance::open_store(dir.path(), grant).is_err());
    }
    #[test]
    fn source_validation_never_rewrites_the_caller() {
        let (g, source, settings) = fixture();
        let before = source.clone();
        g.check_start(
            &json!({"action":"images.design"}),
            &source,
            &settings,
            false,
        )
        .unwrap();
        assert_eq!(source, before);
    }
    #[test]
    fn durable_job_can_finish_its_original_reservation_after_crash() {
        let (g, source, settings) = fixture();
        let dir = tempfile::tempdir().unwrap();
        let session = NativeAcceptance::open_store(dir.path(), g.clone()).unwrap();
        let request = json!({"action":"foundation.run","projectId":"p"});
        let reservation = session
            .reserve(&request, &source, &settings, false)
            .unwrap();
        let job = json!({"id":"original","kind":"workflow","snapshot":{"settings":settings,"nativeAcceptance":session.receipt(&reservation)}});
        drop(session);
        let reopened = NativeAcceptance::open_store(dir.path(), g.clone()).unwrap();
        reopened
            .check_resume(
                &json!({"action":"workflows.resume","projectId":"p","workflowId":"original"}),
                &job,
                &source,
            )
            .unwrap();
        let mut another = g;
        another.grant_id = "different-grant-label".into();
        drop(reopened);
        let rotated = NativeAcceptance::open_store(dir.path(), another).unwrap();
        assert!(rotated
            .reserve(&request, &source, &settings, false)
            .is_err());
    }
    #[test]
    fn live_owner_blocks_takeover_and_orphan_without_any_job_is_audited_and_retryable() {
        let (g, source, settings) = fixture();
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let store = crate::store::Store::open(dir.path().join("project")).unwrap();
        store.create("synthetic", "p").unwrap();
        let session = NativeAcceptance::open_store(&home, g.clone()).unwrap();
        let request = json!({"action":"foundation.run","projectId":"p"});
        session
            .reserve(&request, &source, &settings, false)
            .unwrap();
        assert!(NativeAcceptance::open_store(&home, g.clone()).is_err());
        drop(session);
        let recovered = NativeAcceptance::open_store(&home, g).unwrap();
        recovered.reconcile_unbound(&store).unwrap();
        recovered
            .reserve(&request, &source, &settings, false)
            .unwrap();
        assert_eq!(store.jobs().unwrap().len(), 0);
    }
    #[test]
    fn orphan_with_one_workflow_binds_it_but_ambiguous_or_text_evidence_stays_held() {
        let (g, source, settings) = fixture();
        for count in [0, 1, 2] {
            let dir = tempfile::tempdir().unwrap();
            let home = dir.path().join("home");
            let store = crate::store::Store::open(dir.path().join("project")).unwrap();
            store.create("synthetic", "p").unwrap();
            let session = NativeAcceptance::open_store(&home, g.clone()).unwrap();
            let request = json!({"action":"foundation.run","projectId":"p"});
            let res = session
                .reserve(&request, &source, &settings, false)
                .unwrap();
            let snapshot = json!({"settings":settings,"nativeAcceptance":session.receipt(&res)});
            let mut job = None;
            for _ in 0..count {
                job = Some(
                    store
                        .add_job(
                            "workflow",
                            "foundation.run",
                            "local",
                            "local",
                            &snapshot,
                            "interrupted",
                        )
                        .unwrap(),
                );
            }
            if count == 0 {
                store
                    .add_job(
                        "text",
                        "unknown",
                        "codex",
                        "local",
                        &json!({"stage":"foundation:F2"}),
                        "submission_unknown",
                    )
                    .unwrap();
            }
            drop(session);
            let recovered = NativeAcceptance::open_store(&home, g.clone()).unwrap();
            let result = recovered.reconcile_unbound(&store);
            if count == 1 {
                result.unwrap();
                let id = job.unwrap();
                recovered
                    .check_resume(
                        &json!({"action":"workflows.resume","projectId":"p","workflowId":id}),
                        &store.job(&id).unwrap(),
                        &source,
                    )
                    .unwrap();
            } else {
                assert!(result.is_err());
            }
            assert!(recovered
                .reserve(&request, &source, &settings, false)
                .is_err());
        }
    }
    #[test]
    fn reconciliation_audit_failure_rolls_back_release_and_keeps_claim() {
        let (g, source, settings) = fixture();
        let dir = tempfile::tempdir().unwrap();
        let store = crate::store::Store::open(dir.path().join("project")).unwrap();
        store.create("synthetic", "p").unwrap();
        let session = NativeAcceptance::open_store(&dir.path().join("home"), g).unwrap();
        let request = json!({"action":"foundation.run","projectId":"p"});
        session
            .reserve(&request, &source, &settings, false)
            .unwrap();
        session.conn().unwrap().execute_batch("CREATE TRIGGER fail_audit BEFORE INSERT ON acceptance_events BEGIN SELECT RAISE(ABORT,'synthetic audit failure'); END;").unwrap();
        assert!(session
            .reconcile_unbound(&store)
            .unwrap_err()
            .contains("synthetic audit failure"));
        assert!(session
            .reserve(&request, &source, &settings, false)
            .is_err());
        let state: String = session
            .conn()
            .unwrap()
            .query_row("SELECT state FROM acceptance_runs", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, "reserved");
        assert!(store.jobs().unwrap().is_empty());
    }
}
