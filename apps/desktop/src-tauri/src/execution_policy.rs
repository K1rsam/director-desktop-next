//! Product execution boundary. The first migration build is deliberately offline.
//! Injected test channels exercise the inherited core without enabling user credentials.
use crate::{providers::*, tasks::Channels};
use serde_json::{json, Value};
use std::path::Path;

pub const OFFLINE_ACTIONS: &[&str] = &[
    "getExecutionPolicy",
    "readiness.check",
    "textSdk.status",
    "images.previewPrompt",
    "projects.list",
    "projects.create",
    "projects.archive",
    "projects.restore",
    "projects.delete",
    "files.choose",
    "legacy.inspect",
    "legacy.import",
    "project.import",
    "project.export",
    "workspace.get",
    "units.get",
    "units.versions.list",
    "units.version.restore",
    "units.save",
    "production.direction.save",
    "foundation.choices.list",
    "foundation.choice.confirm",
    "assets.get",
    "assets.usage",
    "assets.save",
    "assets.delete",
    "assets.restore",
    "jobs.detail",
    "jobs.close_unknown",
    "source.import",
    "source.resolve",
    "source.reparse",
    "media.import",
    "media.adopt",
    "media.adopt_batch",
    "images.ensure_current",
    "videos.adopt",
    "settings.save",
    "backup.create",
    "backup.restore",
];
pub const CREDENTIAL_FIELDS: &[&str] = &["bhToken", "chromeToken", "textApiKey"];
pub fn report() -> Value {
    json!({"version":"execution-policy/v1", "mode":"offline_prepare_only",
        "liveExecutionAllowed":false, "stage3CreativePackStatus":"inactive",
        "allowedActions":OFFLINE_ACTIONS,
        "reasons":["真实生成保持关闭；本机只读连接检查须由用户单独点击，云端与隔离探针不读取宿主账户。", "第三阶段原件已核对一致，运行适配仍待验证。"],
        "textBackends":[{"id":"codex_cli","status":"preserved_not_enabled"},{"id":"codex_sdk","status":"offline_contract_only"}],
        "localAcceptance":{"backend":"codex_cli","model":"gpt-6.1-sol","reasoning":"medium"}})
}
pub fn check(request: &Value) -> Result<(), String> {
    let action = request["action"].as_str().unwrap_or("");
    if !OFFLINE_ACTIONS.contains(&action) {
        return Err(format!(
            "OFFLINE_EXECUTION_BLOCKED: {action} 需要单独启用并验证的本地执行入口"
        ));
    }
    if action == "settings.save"
        && CREDENTIAL_FIELDS.iter().any(|field| {
            request["settings"]
                .get(field)
                .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
        })
    {
        return Err("OFFLINE_CREDENTIALS_BLOCKED: 离线工程不保存或读取账户凭据".into());
    }
    Ok(())
}
pub struct OfflineChannels;
fn blocked<T>() -> ProviderResult<T> {
    Err(ProviderError::new(
        "OFFLINE_EXECUTION_BLOCKED",
        "离线工程不连接渠道或启动生成。",
    ))
}
impl Channels for OfflineChannels {
    fn capabilities(&self, _: &ProviderConfig) -> ProviderResult<Capabilities> {
        blocked()
    }
    fn prepare(
        &self,
        _: &ProviderConfig,
        _: &GenerationRequest,
    ) -> ProviderResult<PreparedRequest> {
        blocked()
    }
    fn submit(&self, _: &ProviderConfig, _: &PreparedRequest) -> ProviderResult<SubmitReceipt> {
        blocked()
    }
    fn query(&self, _: &ProviderConfig, _: &str) -> ProviderResult<QueryResult> {
        blocked()
    }
    fn download(&self, _: &ProviderConfig, _: &str, _: &Path) -> ProviderResult<DownloadReceipt> {
        blocked()
    }
    fn reconcile(&self, _: &ProviderConfig, _: &str) -> ProviderResult<QueryResult> {
        blocked()
    }
}

// Stage3 remains structurally closed until the verified Mac runtime adapter is integrated.
// No mutable settings flag or provenance-only manifest can bypass this code gate.
pub fn creative_runtime_ready() -> bool {
    false
}
pub const NATIVE_TEXT_ACTIONS: &[&str] = &[
    "foundation.run",
    "images.design",
    "workflows.resume",
    "textCli.status",
];
pub fn check_native_action(request: &Value) -> Result<(), String> {
    let action = request["action"].as_str().unwrap_or("");
    if OFFLINE_ACTIONS.contains(&action) {
        return check(request);
    }
    if NATIVE_TEXT_ACTIONS.contains(&action) {
        return Ok(());
    }
    if ["storyboard.run", "storyboard.decision.resolve"].contains(&action)
        && creative_runtime_ready()
    {
        return Ok(());
    }
    Err(format!(
        "NATIVE_ACCEPTANCE_SCOPE_BLOCKED: {action} 未在本轮有界入口开放"
    ))
}
pub fn native_report(acceptance: &crate::native_acceptance::NativeAcceptance) -> Value {
    let mut allowed = OFFLINE_ACTIONS.to_vec();
    allowed.extend_from_slice(NATIVE_TEXT_ACTIONS);
    if creative_runtime_ready() {
        allowed.extend_from_slice(&["storyboard.run", "storyboard.decision.resolve"]);
    }
    json!({"version":"execution-policy/v2","mode":"native_cli_acceptance","liveExecutionAllowed":true,
        "stage3CreativePackStatus":if creative_runtime_ready(){"verified_active"}else{"inactive"},
        "allowedActions":allowed,"acceptance":acceptance.status(creative_runtime_ready()),
        "credentialBackend":crate::platform::credential_backend_status(),
        "reasons":["仅对显式授权文件哈希绑定的10集原文执行第一/二阶段文字工作。","CLI固定gpt-6.1-sol / medium；每阶段只有一个新工作流，恢复保留原任务。","Stage3适配未通过前保持关闭；SDK真实调用、未核验免费图片和真实视频均关闭。"]})
}
