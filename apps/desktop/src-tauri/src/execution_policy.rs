//! Product execution boundary. The first migration build is deliberately offline.
//! Injected test channels exercise the inherited core without enabling user credentials.
use crate::{providers::*, tasks::Channels};
use serde_json::{json, Value};
use std::path::Path;

pub const OFFLINE_ACTIONS: &[&str] = &[
    "getExecutionPolicy",
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
        "reasons":["云端离线工程验收不调用模型、渠道或宿主账户。", "第三阶段候选正文等待实际采用的 Mac 运行包核对。"],
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
