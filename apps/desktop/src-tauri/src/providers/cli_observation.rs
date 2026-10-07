//! Optional Canvas-only process facts. No streams, arguments, paths or environment.
use serde_json::{json, Value};
use std::{
    process::ExitStatus,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone)]
pub(crate) struct CliObservation {
    start: Instant,
    data: Arc<Mutex<Value>>,
}
impl CliObservation {
    pub(crate) fn new() -> Self {
        Self {
            start: Instant::now(),
            data: Arc::new(Mutex::new(json!({
                "phase":"created","pid":null,"elapsedMs":0,"milestones":{"created":0},
                "stdout":{"bytes":0,"eof":false,"readError":false,"firstByteMs":null,"eofMs":null},
                "stderr":{"bytes":0,"eof":false,"readError":false,"firstByteMs":null,"eofMs":null},
                "processExit":null,"timeout":null,"preCleanup":null,
                "cleanup":{"killRequested":false,"reapConfirmed":null},
                "controlTicks":0,"maxControlGapMs":0,"lastControlTickMs":null,"budgetMs":null
            }))),
        }
    }
    fn ms(&self) -> u64 {
        self.start.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
    fn update(&self, f: impl FnOnce(&mut Value, u64)) {
        if let Ok(mut data) = self.data.lock() {
            f(&mut data, self.ms());
        }
    }
    pub(crate) fn mark(&self, phase: &'static str) {
        self.update(|d, t| {
            d["phase"] = json!(phase);
            d["milestones"][phase] = json!(t);
        });
    }
    pub(crate) fn budget(&self, ms: u64) {
        self.update(|d, _| d["budgetMs"] = json!(ms));
    }
    pub(crate) fn spawned(&self, pid: Option<u32>) {
        self.update(|d, _| d["pid"] = json!(pid));
        self.mark("spawn_end");
    }
    pub(crate) fn pipe(&self, stream: &'static str, count: usize, eof: bool, error: bool) {
        self.update(|d, t| {
            let p = &mut d[stream];
            if count > 0 {
                p["bytes"] = json!(p["bytes"]
                    .as_u64()
                    .unwrap_or(0)
                    .saturating_add(count as u64));
                if p["firstByteMs"].is_null() {
                    p["firstByteMs"] = json!(t);
                }
            }
            if eof {
                p["eof"] = json!(true);
                p["eofMs"] = json!(t);
            }
            if error {
                p["readError"] = json!(true);
            }
        });
    }
    fn exit_value(status: ExitStatus, t: u64) -> Value {
        #[cfg(unix)]
        let signal = {
            use std::os::unix::process::ExitStatusExt;
            status.signal()
        };
        #[cfg(not(unix))]
        let signal: Option<i32> = None;
        json!({"code":status.code(),"signal":signal,"success":status.success(),"observedAtMs":t})
    }
    pub(crate) fn exited(&self, status: ExitStatus) {
        self.update(|d, t| {
            d["processExit"] = Self::exit_value(status, t);
            d["milestones"]["exit_observed"] = json!(t);
            d["cleanup"]["reapConfirmed"] = json!(true);
        });
    }
    pub(crate) fn tick(&self) {
        self.update(|d, t| {
            let prior = d["lastControlTickMs"].as_u64().unwrap_or(t);
            let gap = t.saturating_sub(prior);
            d["maxControlGapMs"] = json!(d["maxControlGapMs"].as_u64().unwrap_or(0).max(gap));
            d["lastControlTickMs"] = json!(t);
            d["controlTicks"] = json!(d["controlTicks"].as_u64().unwrap_or(0).saturating_add(1));
        });
    }
    pub(crate) fn timed_out(&self, kind: &'static str, overshoot_ms: u64, exit_observed: bool) {
        self.update(|d,t|{let gap=t.saturating_sub(d["lastControlTickMs"].as_u64().unwrap_or(t));d["maxControlGapMs"]=json!(d["maxControlGapMs"].as_u64().unwrap_or(0).max(gap));d["timeout"]=json!({"kind":kind,"elapsedMs":t,"overshootMs":overshoot_ms,"exitObserved":exit_observed});});
        self.mark("timeout");
    }
    pub(crate) fn pre_cleanup(&self, result: &std::io::Result<Option<ExitStatus>>) {
        self.update(|d, t| {
            d["preCleanup"] = match result {
                Ok(Some(s)) => {
                    let e = Self::exit_value(*s, t);
                    if d["processExit"].is_null() {
                        d["processExit"] = e.clone();
                        d["milestones"]["exit_observed"] = json!(t);
                    }
                    json!({"state":"exited","exitCode":e["code"],"signal":e["signal"]})
                }
                Ok(None) => json!({"state":"running","exitCode":null,"signal":null}),
                Err(_) => json!({"state":"query_failed","exitCode":null,"signal":null}),
            };
        });
    }
    pub(crate) fn kill_requested(&self) {
        self.update(|d, _| d["cleanup"]["killRequested"] = json!(true));
    }
    pub(crate) fn cleanup_done(&self, confirmed: bool) {
        self.update(|d, _| d["cleanup"]["reapConfirmed"] = json!(confirmed));
        self.mark("cleanup_end");
    }
    pub(crate) fn snapshot(&self) -> Value {
        let mut d = self
            .data
            .lock()
            .map(|d| d.clone())
            .unwrap_or_else(|_| json!({"phase":"observation_unavailable"}));
        d["elapsedMs"] = json!(self.ms());
        d.as_object_mut().unwrap().remove("lastControlTickMs");
        d
    }
}
