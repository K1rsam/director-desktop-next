//! Opt-in startup evidence for the isolated --verify-startup probe only.
//! This module has no account, environment, model, script-body or network access.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::Path,
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub const MAX_EVENTS: usize = 144;
const NORMAL_EVENT_LIMIT: usize = 128;
const MAX_LINE_BYTES: usize = 1024;

#[derive(Clone, Copy)]
pub enum Stage {
    MainEntered,
    ArgumentsAccepted,
    DesktopBuildStarted,
    DesktopBuildFailed,
    SetupEntered,
    EngineReady,
    FixtureSeeded,
    MediaScopeReady,
    SetupComplete,
    PageLoaded,
    ProbeInjected,
    ProbeInjectionFailed,
    Frontend,
    ReportReceived,
    ReportPersisted,
    WatchdogTimeout,
    ExitRequested,
    ShutdownStarted,
    ShutdownFinished,
    NativeExitIssued,
    RustPanic,
}
impl Stage {
    fn name(self) -> &'static str {
        match self {
            Self::MainEntered => "main_entered",
            Self::ArgumentsAccepted => "arguments_accepted",
            Self::DesktopBuildStarted => "desktop_build_started",
            Self::DesktopBuildFailed => "desktop_build_failed",
            Self::SetupEntered => "setup_entered",
            Self::EngineReady => "engine_ready",
            Self::FixtureSeeded => "fixture_seeded",
            Self::MediaScopeReady => "media_scope_ready",
            Self::SetupComplete => "setup_complete",
            Self::PageLoaded => "page_loaded",
            Self::ProbeInjected => "probe_injected",
            Self::ProbeInjectionFailed => "probe_injection_failed",
            Self::Frontend => "frontend",
            Self::ReportReceived => "report_received",
            Self::ReportPersisted => "report_persisted",
            Self::WatchdogTimeout => "watchdog_timeout",
            Self::ExitRequested => "exit_requested",
            Self::ShutdownStarted => "shutdown_started",
            Self::ShutdownFinished => "shutdown_finished",
            Self::NativeExitIssued => "native_exit_issued",
            Self::RustPanic => "rust_panic",
        }
    }
    fn terminal(self) -> bool {
        matches!(
            self,
            Self::ReportReceived
                | Self::ReportPersisted
                | Self::WatchdogTimeout
                | Self::ExitRequested
                | Self::ShutdownStarted
                | Self::ShutdownFinished
                | Self::NativeExitIssued
                | Self::RustPanic
        )
    }
}
struct Journal {
    file: File,
    sequence: usize,
    normal_events: usize,
}
pub struct StartupDiagnostics {
    journal: Mutex<Journal>,
    started: Instant,
}
impl StartupDiagnostics {
    /// Creates a separate append-only journal. Existing evidence is never replaced.
    /// The first record is flushed before any Tauri/AppKit/WebView initialization.
    pub fn start(report: &Path) -> io::Result<Arc<Self>> {
        if !report.is_absolute() || report.file_name().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "absolute report file required",
            ));
        }
        match fs::symlink_metadata(report) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "report already exists",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let parent = report
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "report parent missing"))?;
        // The caller's explicit new report directory is the entire diagnostic scope.
        fs::create_dir_all(parent)?;
        let path = report.with_extension("events.ndjson");
        let file = OpenOptions::new().create_new(true).write(true).open(path)?;
        let result = Arc::new(Self {
            journal: Mutex::new(Journal {
                file,
                sequence: 0,
                normal_events: 0,
            }),
            started: Instant::now(),
        });
        result.write(Stage::MainEntered, None, None, false)?;
        Ok(result)
    }
    pub fn mark(&self, stage: Stage) {
        let _ = self.write(stage, None, None, false);
    }
    pub fn frontend(&self, checkpoint: &str) -> Result<(), &'static str> {
        let fixed = match checkpoint {
            "started" => "started",
            "project_round_trip" => "project_round_trip",
            "asset_protocol" => "asset_protocol",
            "stage_one" => "stage_one",
            "source_review" => "source_review",
            "stage_three" => "stage_three",
            "settings" => "settings",
            "asset_lifecycle" => "asset_lifecycle",
            "unit_editor" => "unit_editor",
            "project_lifecycle" => "project_lifecycle",
            "inline_create" => "inline_create",
            "completed" => "completed",
            "failed" => "failed",
            _ => return Err("Unknown startup checkpoint"),
        };
        self.write(Stage::Frontend, Some(fixed), None, false)
            .map_err(|_| "Startup checkpoint unavailable")
    }
    /// Verification mode only. Payloads and environment are deliberately omitted.
    /// try_lock prevents a panic while logging from deadlocking the panic hook.
    pub fn install_panic_hook(self: &Arc<Self>) {
        let diagnostics = self.clone();
        std::panic::set_hook(Box::new(move |info| {
            let location = info.location().map(|l| (l.file(), l.line(), l.column()));
            let _ = diagnostics.write(Stage::RustPanic, None, location, true);
            let _ = writeln!(io::stderr(), "Isolated startup verification encountered a Rust panic; payload omitted; inspect the startup events journal");
        }));
    }
    fn write(
        &self,
        stage: Stage,
        checkpoint: Option<&'static str>,
        location: Option<(&str, u32, u32)>,
        nonblocking: bool,
    ) -> io::Result<()> {
        let mut journal = if nonblocking {
            self.journal
                .try_lock()
                .map_err(|_| io::Error::new(io::ErrorKind::WouldBlock, "diagnostic journal busy"))?
        } else {
            self.journal
                .lock()
                .map_err(|_| io::Error::other("diagnostic journal poisoned"))?
        };
        if journal.sequence >= MAX_EVENTS
            || (!stage.terminal() && journal.normal_events >= NORMAL_EVENT_LIMIT)
        {
            return Err(io::Error::other("diagnostic event limit reached"));
        }
        let unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut record=format!("{{\"schema\":\"startup-diagnostics/v1\",\"sequence\":{},\"stage\":\"{}\",\"pid\":{},\"platform\":\"{}\",\"unixMillis\":{},\"elapsedMillis\":{}",journal.sequence+1,stage.name(),std::process::id(),std::env::consts::OS,unix_ms,self.started.elapsed().as_millis());
        if let Some(checkpoint) = checkpoint {
            record.push_str(&format!(",\"checkpoint\":\"{checkpoint}\""));
        }
        if let Some((file, line, column)) = location {
            // Compiler location only; omit directories, payload, stack locals and args.
            let file = file.rsplit(['/', '\\']).next().unwrap_or("unknown");
            let safe = file.len() <= 96
                && file
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c));
            record.push_str(&format!(",\"panicLocation\":{{\"file\":\"{}\",\"line\":{line},\"column\":{column}}},\"panicPayloadRecorded\":false",if safe {file}else{"unknown"}));
        }
        record.push_str("}\n");
        if record.len() > MAX_LINE_BYTES {
            return Err(io::Error::other("diagnostic record limit reached"));
        }
        journal.file.write_all(record.as_bytes())?;
        journal.file.sync_data()?;
        journal.sequence += 1;
        if !stage.terminal() {
            journal.normal_events += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        process::Command,
        sync::atomic::{AtomicU64, Ordering},
    };
    fn temp() -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "director-startup-diagnostics-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test]
    fn first_marker_is_durable_and_existing_evidence_is_never_replaced() {
        let dir = temp();
        let report = dir.join("startup.json");
        let journal = report.with_extension("events.ndjson");
        let d = StartupDiagnostics::start(&report).unwrap();
        let first = fs::read_to_string(&journal).unwrap();
        assert!(first.contains("\"stage\":\"main_entered\""));
        assert!(!report.exists());
        assert!(StartupDiagnostics::start(&report).is_err());
        assert_eq!(fs::read_to_string(&journal).unwrap(), first);
        d.mark(Stage::SetupEntered);
        drop(d);
        assert_eq!(fs::read_to_string(&journal).unwrap().lines().count(), 2);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn dynamic_values_are_rejected_and_event_flood_reserves_terminal_space() {
        let dir = temp();
        let report = dir.join("startup.json");
        let d = StartupDiagnostics::start(&report).unwrap();
        assert!(d.frontend("PRIVATE_SENTINEL credential or script").is_err());
        for _ in 0..1000 {
            let _ = d.frontend("started");
        }
        d.mark(Stage::ReportReceived);
        d.mark(Stage::ReportPersisted);
        d.mark(Stage::NativeExitIssued);
        let text = fs::read_to_string(report.with_extension("events.ndjson")).unwrap();
        assert_eq!(text.lines().count(), NORMAL_EVENT_LIMIT + 3);
        assert!(!text.contains("PRIVATE_SENTINEL"));
        assert!(text.contains("native_exit_issued"));
        assert!(text.len() <= MAX_EVENTS * MAX_LINE_BYTES);
        drop(d);
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn relative_or_completed_reports_fail_before_logging() {
        assert!(StartupDiagnostics::start(Path::new("relative.json")).is_err());
        let dir = temp();
        let report = dir.join("startup.json");
        fs::write(&report, b"preserve").unwrap();
        assert!(StartupDiagnostics::start(&report).is_err());
        assert!(!report.with_extension("events.ndjson").exists());
        assert_eq!(fs::read(&report).unwrap(), b"preserve");
        fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn diagnostic_child_fixture() {
        let Some(dir) = std::env::var_os("DIRECTOR_SYNTHETIC_DIAGNOSTIC_CHILD") else {
            return;
        };
        let d = StartupDiagnostics::start(&PathBuf::from(dir).join("startup.json")).unwrap();
        d.install_panic_hook();
        d.mark(Stage::SetupEntered);
        if std::env::var_os("DIRECTOR_SYNTHETIC_PANIC").is_some() {
            panic!("PRIVATE_SENTINEL must never enter the panic journal or stderr");
        }
        std::process::exit(87);
    }
    #[test]
    fn panic_and_abrupt_exit_preserve_markers_without_payload() {
        for panic in [false, true] {
            let dir = temp();
            let mut cmd = Command::new(std::env::current_exe().unwrap());
            cmd.args([
                "--exact",
                "startup_diagnostics::tests::diagnostic_child_fixture",
                "--nocapture",
            ]);
            // Standalone rustc --test uses tests::, Cargo uses startup_diagnostics::tests::.
            if std::env::current_exe()
                .unwrap()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("startup-diagnostics-standalone")
            {
                cmd = Command::new(std::env::current_exe().unwrap());
                cmd.args(["--exact", "tests::diagnostic_child_fixture", "--nocapture"]);
            }
            cmd.env("DIRECTOR_SYNTHETIC_DIAGNOSTIC_CHILD", &dir);
            if panic {
                cmd.env("DIRECTOR_SYNTHETIC_PANIC", "1");
            }
            let output = cmd.output().unwrap();
            assert!(!output.status.success());
            let text = fs::read_to_string(dir.join("startup.events.ndjson")).unwrap();
            assert!(text.contains("setup_entered"));
            assert_eq!(text.contains("rust_panic"), panic);
            assert!(!text.contains("PRIVATE_SENTINEL"));
            assert!(!String::from_utf8_lossy(&output.stderr).contains("PRIVATE_SENTINEL"));
            assert!(!String::from_utf8_lossy(&output.stdout).contains("PRIVATE_SENTINEL"));
            if panic {
                assert!(text.contains("panicPayloadRecorded\":false"));
                assert!(text.contains("startup_diagnostics.rs"));
            }
            fs::remove_dir_all(dir).unwrap();
        }
    }
}
