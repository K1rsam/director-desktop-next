//! Versioned transport seam. It does not replace the existing SavedTextExecutor.
//! CLI and official SDK share frozen execution identity; there is no automatic fallback.
use crate::{
    pipeline::{digest, TextRequest},
    store::{atomic_json, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Backend {
    CodexCli,
    CodexSdk,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttemptState {
    Prepared,
    Dispatching,
    Observed,
    Completed,
    Rejected,
    SubmissionUnknown,
    CancelRequested,
    Stopped,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenAttempt {
    pub version: String,
    pub attempt_id: String,
    pub workflow_id: String,
    pub logical_node_id: String,
    pub backend: Backend,
    pub request: TextRequest,
    pub semantic_request_hash: String,
    pub execution_fingerprint: String,
    pub runtime_identity: Value,
    pub attempt_envelope_hash: String,
    pub dispatch_started: bool,
    pub state: AttemptState,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub last_sequence: u64,
    pub raw_result: Option<Value>,
}
impl FrozenAttempt {
    pub fn prepare(
        attempt_id: &str,
        workflow_id: &str,
        logical_node_id: &str,
        backend: Backend,
        request: TextRequest,
        runtime_identity: &Value,
    ) -> Result<Self> {
        if [
            attempt_id,
            workflow_id,
            logical_node_id,
            &request.model,
            &request.reasoning,
        ]
        .iter()
        .any(|x| x.trim().is_empty())
        {
            return Err("冻结执行缺少任务、模型或推理身份".into());
        }
        let semantic = json!({"stage":request.stage,"skill":request.skill,"input":request.input,
            "schema":request.schema,"ruleSnapshot":request.rule_snapshot});
        let fingerprint = json!({"version":"execution-fingerprint/v1","backend":backend,
            "model":request.model,"reasoning":request.reasoning,"runtime":runtime_identity});
        let semantic_request_hash = digest(&semantic.to_string());
        let execution_fingerprint = digest(&fingerprint.to_string());
        let envelope = json!({"version":"attempt-identity/v1","attemptId":attempt_id,
            "workflowId":workflow_id,"logicalNodeId":logical_node_id,
            "semanticRequestHash":semantic_request_hash,"executionFingerprint":execution_fingerprint});
        Ok(Self {
            version: "text-attempt/v2".into(),
            attempt_id: attempt_id.into(),
            workflow_id: workflow_id.into(),
            logical_node_id: logical_node_id.into(),
            backend,
            semantic_request_hash,
            execution_fingerprint,
            runtime_identity: runtime_identity.clone(),
            attempt_envelope_hash: digest(&envelope.to_string()),
            dispatch_started: false,
            request,
            state: AttemptState::Prepared,
            thread_id: None,
            turn_id: None,
            last_sequence: 0,
            raw_result: None,
        })
    }
    pub fn persist(&self, path: &Path) -> Result<()> {
        self.validate_identity()?;
        let payload = serde_json::to_value(self).map_err(|e| e.to_string())?;
        atomic_json(
            path,
            &json!({"format":"frozen-attempt-envelope/v1",
            "sha256":digest(&payload.to_string()),"payload":payload}),
        )
    }
    pub fn load(path: &Path) -> Result<Self> {
        let record: Value =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if record["format"] != "frozen-attempt-envelope/v1"
            || record["sha256"] != digest(&record["payload"].to_string())
        {
            return Err("冻结执行磁盘封套缺失或校验失败，不猜测恢复或重发".into());
        }
        let value: Self =
            serde_json::from_value(record["payload"].clone()).map_err(|e| e.to_string())?;
        value.validate_identity()?;
        Ok(value)
    }
    pub fn validate_identity(&self) -> Result<()> {
        let expected = Self::prepare(
            &self.attempt_id,
            &self.workflow_id,
            &self.logical_node_id,
            self.backend.clone(),
            self.request.clone(),
            &self.runtime_identity,
        )?;
        if self.version != expected.version
            || self.semantic_request_hash != expected.semantic_request_hash
            || self.execution_fingerprint != expected.execution_fingerprint
            || self.attempt_envelope_hash != expected.attempt_envelope_hash
        {
            return Err("冻结请求或执行身份哈希不符，保留证据且不执行".into());
        }
        let prior_evidence = self.dispatch_started
            || self.thread_id.is_some()
            || self.turn_id.is_some()
            || self.last_sequence != 0
            || self.raw_result.is_some();
        if self.state == AttemptState::Prepared && prior_evidence {
            return Err("prepared 执行携带既有派发或回执，禁止重发".into());
        }
        if self.state != AttemptState::Prepared && !self.dispatch_started {
            return Err("执行状态缺少原始派发标记，不猜测恢复".into());
        }
        Ok(())
    }
    pub fn begin_dispatch(&mut self, backend: Backend) -> Result<()> {
        self.validate_identity()?;
        if backend != self.backend {
            return Err("执行入口已冻结，不允许跨入口自动重发".into());
        }
        if self.state != AttemptState::Prepared {
            return Err("此执行可能已受理，只能核对原任务身份".into());
        }
        self.state = AttemptState::Dispatching;
        self.dispatch_started = true;
        Ok(())
    }
    pub fn observe(&mut self, event: Observation) -> Result<bool> {
        self.validate_identity()?;
        if event
            .thread_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty())
            || event
                .turn_id
                .as_ref()
                .is_some_and(|id| id.trim().is_empty())
        {
            return Err("回执 thread/turn 身份为空".into());
        }
        if event.attempt_id != self.attempt_id {
            return Err("回执不属于此执行".into());
        }
        if event.sequence <= self.last_sequence {
            return Ok(false);
        }
        if self
            .thread_id
            .as_ref()
            .is_some_and(|id| event.thread_id.as_ref().is_some_and(|next| id != next))
            || self
                .turn_id
                .as_ref()
                .is_some_and(|id| event.turn_id.as_ref().is_some_and(|next| id != next))
        {
            return Err("原 thread/turn 身份变化，已阻断自动采用".into());
        }
        if matches!(
            self.state,
            AttemptState::Completed | AttemptState::Rejected | AttemptState::Stopped
        ) {
            return Err("终态执行不接受后到事件覆盖".into());
        }
        if self.state == AttemptState::Prepared {
            return Err("未派发执行不能接收运行回执".into());
        }
        if event.kind == ObservationKind::Completed && event.raw_result.is_none() {
            return Err("完成事件缺少原始结果".into());
        }
        self.thread_id = event.thread_id.or(self.thread_id.clone());
        self.turn_id = event.turn_id.or(self.turn_id.clone());
        self.last_sequence = event.sequence;
        self.state = match event.kind {
            ObservationKind::Started => AttemptState::Observed,
            ObservationKind::Completed => AttemptState::Completed,
            ObservationKind::SubmissionUnknown => AttemptState::SubmissionUnknown,
            ObservationKind::Stopped => AttemptState::Stopped,
        };
        if let Some(raw) = event.raw_result {
            self.raw_result = Some(raw);
        }
        Ok(true)
    }
    /// Compatibility only; this never declares output domain-accepted or publishable.
    pub fn has_same_execution_identity(&self, other: &Self) -> bool {
        self.validate_identity().is_ok()
            && other.validate_identity().is_ok()
            && self.semantic_request_hash == other.semantic_request_hash
            && self.execution_fingerprint == other.execution_fingerprint
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    Started,
    Completed,
    SubmissionUnknown,
    Stopped,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub attempt_id: String,
    pub sequence: u64,
    pub kind: ObservationKind,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub raw_result: Option<Value>,
}

/// A transport completion only means bytes arrived. Domain validation and publication
/// remain in SavedTextExecutor / pipeline, including unresolved and repair budgets.
#[async_trait::async_trait]
pub trait TextTransport: Send + Sync {
    async fn inspect(&self) -> Result<Value>;
    async fn execute(&self, attempt: &FrozenAttempt) -> Result<Observation>;
    async fn reconcile(&self, attempt: &FrozenAttempt) -> Result<Observation>;
    async fn cancel(&self, attempt: &FrozenAttempt) -> Result<Observation>;
}

/// Honest placeholder until an official SDK sidecar and its runtime are pinned.
/// It is not presented as an implemented SDK execution backend.
pub struct UnavailableSdkTransport;
#[async_trait::async_trait]
impl TextTransport for UnavailableSdkTransport {
    async fn inspect(&self) -> Result<Value> {
        Ok(
            json!({"backend":"codex_sdk","available":false,"liveCallsAllowed":false,
            "reason":"Official SDK sidecar/package/runtime verification is pending"}),
        )
    }
    async fn execute(&self, _: &FrozenAttempt) -> Result<Observation> {
        Err("SDK_NOT_IMPLEMENTED: 不回退 CLI".into())
    }
    async fn reconcile(&self, _: &FrozenAttempt) -> Result<Observation> {
        Err("SDK_RECOVERY_UNVERIFIED: 保留原执行，不启动新 turn".into())
    }
    async fn cancel(&self, _: &FrozenAttempt) -> Result<Observation> {
        Err("SDK_CANCEL_UNVERIFIED: 未确认底层进程停止".into())
    }
}
