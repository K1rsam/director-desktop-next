//! Canvas-only, memory-only output classification. Never publishes free-form CLI output.
use super::{failure, ReadCommand};
use crate::providers::ProviderResult;
use serde_json::{json, Map, Value};

pub(crate) struct ProcessOutput {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub local_log_write_restricted: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

// These are display allowlists, not inferred causes or executable instructions.
// Unknown upstream enum values stay explicitly unrecognized, never echoed.
fn allowed(key: &str, value: &str) -> bool {
    match key {
        "code" => value == "service.10000",
        "class" => ["auth_required", "access_denied"].contains(&value),
        "requiredAction" => ["none", "login"].contains(&value),
        "reason" => [
            "oauth_scope_insufficient",
            "dns_resolution_failed",
            "network_permission_denied",
            "credential_store_permission_denied",
            "credential_store_unavailable",
            "credential_store_corrupt",
            "network_timeout",
            "auth_dependency_failed",
        ]
        .contains(&value),
        _ => false,
    }
}

fn safe_error(body: &Value) -> Value {
    let mut result = Map::new();
    for key in ["code", "class", "reason", "requiredAction"] {
        if let Some(value) = body["error"].get(key) {
            result.insert(
                key.into(),
                json!(value
                    .as_str()
                    .filter(|value| allowed(key, value))
                    .unwrap_or("unrecognized")),
            );
        }
    }
    Value::Object(result)
}
fn shape(bytes: &[u8]) -> (&'static str, Option<Value>) {
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return ("empty", None);
    }
    let Ok(body) = serde_json::from_slice::<Value>(bytes) else {
        return ("malformed_json", None);
    };
    if !body.is_object() {
        return ("invalid_envelope", None);
    }
    let envelope_like = ["schemaVersion", "ok", "error"]
        .iter()
        .any(|key| body.get(*key).is_some());
    if !envelope_like {
        return ("invalid_envelope", None);
    }
    if body["schemaVersion"] != "1" {
        return ("unsupported_envelope", Some(body));
    }
    if body["ok"] == false && body["error"].is_object() {
        return ("error_envelope", Some(body));
    }
    if body["ok"] == true && body["data"].is_object() {
        return ("success_envelope", Some(body));
    }
    ("invalid_envelope", Some(body))
}

pub(crate) fn classify(
    command: ReadCommand,
    output: ProcessOutput,
) -> (Value, ProviderResult<Vec<u8>>) {
    let mut diagnostic = json!({
        "termination":if output.signal.is_some(){"signal"}else if output.exit_code.is_some(){"exit"}else{"unknown"},
        "exitCode":output.exit_code,"signal":output.signal,"processSucceeded":output.success,
        "stdoutBytes":output.stdout.len(),"stderrBytes":output.stderr.len(),
        "stdoutShape":"not_parsed","stderrShape":"not_parsed","structuredErrorSource":"none","errors":{}
    });
    // Both streams were already bounded by the common supervisor. A stricter
    // Canvas limit applies before either stream is parsed or any field exported.
    if output.stdout.len() > command.output_limit() || output.stderr.len() > 64 * 1024 {
        return (diagnostic, Err(failure("CANVAS_OUTPUT_LIMIT")));
    }
    let (out_shape, out) = shape(&output.stdout);
    let (err_shape, err) = shape(&output.stderr);
    diagnostic["stdoutShape"] = json!(out_shape);
    diagnostic["stderrShape"] = json!(err_shape);
    if out_shape == "error_envelope" {
        diagnostic["errors"]["stdout"] = safe_error(out.as_ref().unwrap());
    }
    if err_shape == "error_envelope" {
        diagnostic["errors"]["stderr"] = safe_error(err.as_ref().unwrap());
    }
    // Preserve the existing narrow local logger-denial gate without using it to
    // infer authentication or network state. Its raw text never escapes.
    if output.local_log_write_restricted {
        return (diagnostic, Err(failure("CLI_LOCAL_LOG_WRITE_RESTRICTED")));
    }
    // Two structured channels must not silently select a more convenient story.
    // Identical errors are harmless duplication; all other dual envelopes conflict.
    let conflict = out.is_some()
        && err.is_some()
        && !(out_shape == "error_envelope" && err_shape == "error_envelope" && out == err);
    if conflict {
        diagnostic["structuredErrorSource"] = json!("conflict");
        return (diagnostic, Err(failure("CANVAS_OUTPUT_CONFLICT")));
    }
    if out_shape == "error_envelope" {
        diagnostic["structuredErrorSource"] = json!("stdout");
    } else if err_shape == "error_envelope" {
        diagnostic["structuredErrorSource"] = json!("stderr");
    }
    if output.signal.is_some() {
        return (diagnostic, Err(failure("CANVAS_PROCESS_SIGNAL")));
    }
    let chosen_error = if out_shape == "error_envelope" {
        out.as_ref()
    } else if err_shape == "error_envelope" {
        err.as_ref()
    } else {
        None
    };
    if command == ReadCommand::AuthAccount
        && chosen_error.is_some_and(|body| {
            body["error"]["code"] == "service.10000"
                && body["error"]["class"] == "auth_required"
                && body["error"]["requiredAction"] == "login"
                && body["error"]["reason"] == "oauth_scope_insufficient"
        })
    {
        return (diagnostic, Err(failure("CANVAS_OAUTH_SCOPE_INSUFFICIENT")));
    }
    // An explicit structured error is a failure even if the CLI exits zero.
    // Return only that error envelope to the existing typed reason classifier.
    if out_shape == "error_envelope" {
        return (diagnostic, Ok(output.stdout));
    }
    if err_shape == "error_envelope" {
        return (diagnostic, Ok(output.stderr));
    }
    if !output.success {
        return (
            diagnostic,
            Err(failure(if out_shape == "success_envelope" {
                "CANVAS_NONZERO_SUCCESS"
            } else {
                "CANVAS_COMMAND_FAILED"
            })),
        );
    }
    if out_shape == "empty" {
        return (diagnostic, Err(failure("CANVAS_EMPTY_OUTPUT")));
    }
    if err_shape == "success_envelope" {
        return (diagnostic, Err(failure("CANVAS_SUCCESS_STDOUT_MISSING")));
    }
    // The success contract remains stdout-only and is validated again by envelope().
    (diagnostic, Ok(output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn output(exit: i32, stdout: &[u8], stderr: &[u8]) -> ProcessOutput {
        ProcessOutput {
            success: exit == 0,
            exit_code: Some(exit),
            signal: None,
            local_log_write_restricted: false,
            stdout: stdout.to_vec(),
            stderr: stderr.to_vec(),
        }
    }
    const ERROR: &[u8] = br#"{"schemaVersion":"1","ok":false,"error":{"code":"AUTH_DEPENDENCY_FAILED","class":"dependency","reason":"network_timeout","requiredAction":"check_network","message":"PRIVATE_TOKEN /PRIVATE_PATH userId=123456789","token":"PRIVATE_TOKEN"},"meta":{"requestId":"PRIVATE_ID"}}"#;
    const OK: &[u8] = br#"{"schemaVersion":"1","ok":true,"data":{"userId":123456789}}"#;
    #[test]
    fn stderr_failure_is_parsed_without_exporting_raw_or_inferencing_from_exit() {
        let (meta, result) = classify(ReadCommand::AuthAccount, output(11, b"", ERROR));
        assert_eq!(meta["exitCode"], 11);
        assert_eq!(meta["stdoutBytes"], 0);
        assert_eq!(meta["stderrBytes"], ERROR.len());
        assert_eq!(meta["structuredErrorSource"], "stderr");
        assert_eq!(meta["errors"]["stderr"]["reason"], "network_timeout");
        assert_eq!(
            super::super::envelope(&result.unwrap()).unwrap_err().code,
            "CANVAS_NETWORK_TIMEOUT"
        );
        for secret in [
            "PRIVATE_",
            "123456789",
            "userId",
            "token",
            "message",
            "requestId",
        ] {
            assert!(!meta.to_string().contains(secret), "{secret}");
        }
    }
    #[test]
    fn malformed_and_untrusted_text_never_assign_tls_auth_network_cause() {
        for text in [
            b"TLS auth failed PRIVATE_TOKEN".as_slice(),
            b"{not json",
            b"[1,2]",
            br#"{"schemaVersion":"2","ok":false,"error":{"reason":"network_timeout"}}"#,
        ] {
            let (meta, result) = classify(ReadCommand::ImageModels, output(12, b"", text));
            assert_eq!(result.unwrap_err().code, "CANVAS_COMMAND_FAILED");
            assert_eq!(meta["errors"], json!({}));
            assert!(!meta.to_string().contains("PRIVATE"));
        }
    }
    #[test]
    fn unrecognized_error_enums_and_extra_objects_are_not_echoed() {
        let body=br#"{"schemaVersion":"1","ok":false,"error":{"code":"PRIVATE_TOKEN","class":"PRIVATE_CLASS","requiredAction":{"password":"PRIVATE_PASSWORD"},"reason":"PRIVATE_REASON","localStorage":{"directory":"PRIVATE_PATH"}}}"#;
        let (meta, result) = classify(ReadCommand::AuthAccount, output(11, b"", body));
        assert!(result.is_ok());
        assert!(!meta.to_string().contains("PRIVATE"));
        assert_eq!(
            meta["errors"]["stderr"],
            json!({"code":"unrecognized","class":"unrecognized","requiredAction":"unrecognized","reason":"unrecognized"})
        );
    }
    #[test]
    fn conflicting_streams_fail_closed_even_with_success_stdout() {
        for (stdout, stderr) in [(OK, ERROR), (ERROR, OK), (OK, OK)] {
            let (meta, result) = classify(ReadCommand::AuthAccount, output(0, stdout, stderr));
            assert_eq!(result.unwrap_err().code, "CANVAS_OUTPUT_CONFLICT");
            assert_eq!(meta["structuredErrorSource"], "conflict");
            assert!(!meta.to_string().contains("123456789"));
        }
        let mut other: Value = serde_json::from_slice(ERROR).unwrap();
        other["error"]["reason"] = json!("dns_resolution_failed");
        assert_eq!(
            classify(
                ReadCommand::AuthAccount,
                output(11, ERROR, &serde_json::to_vec(&other).unwrap())
            )
            .1
            .unwrap_err()
            .code,
            "CANVAS_OUTPUT_CONFLICT"
        );
        assert!(classify(ReadCommand::AuthAccount, output(11, ERROR, ERROR))
            .1
            .is_ok());
    }
    #[test]
    fn success_stdout_cannot_hide_invalid_or_unsupported_stderr_envelopes() {
        for stderr in [
            br#"{"schemaVersion":"1","ok":false,"error":"invalid PRIVATE"}"#.as_slice(),
            br#"{"schemaVersion":"2","ok":false,"error":{"message":"PRIVATE"}}"#,
            br#"{"error":{"code":"PRIVATE"}}"#,
        ] {
            let (meta, result) = classify(ReadCommand::AuthAccount, output(0, OK, stderr));
            assert_eq!(result.unwrap_err().code, "CANVAS_OUTPUT_CONFLICT");
            assert_eq!(meta["structuredErrorSource"], "conflict");
            assert_eq!(meta["errors"], json!({}));
            assert!(!meta.to_string().contains("PRIVATE"));
        }
    }
    #[test]
    fn zero_exit_empty_or_stderr_success_never_succeeds() {
        for stderr in [b"".as_slice(), b"warning PRIVATE_TOKEN", OK] {
            assert_eq!(
                classify(ReadCommand::AuthAccount, output(0, b"", stderr))
                    .1
                    .unwrap_err()
                    .code,
                "CANVAS_EMPTY_OUTPUT"
            );
        }
        assert!(super::super::envelope(
            &classify(ReadCommand::AuthAccount, output(0, b"", ERROR))
                .1
                .unwrap()
        )
        .is_err());
        assert_eq!(
            classify(ReadCommand::AuthAccount, output(12, OK, b""))
                .1
                .unwrap_err()
                .code,
            "CANVAS_NONZERO_SUCCESS"
        );
    }
    #[test]
    fn exact_scope_error_is_account_specific_and_never_an_automatic_login_action() {
        let body=br#"{"schemaVersion":"1","ok":false,"error":{"code":"service.10000","class":"auth_required","requiredAction":"login","reason":"oauth_scope_insufficient","message":"PRIVATE_SCOPE_MESSAGE"}}"#;
        let (meta, result) = classify(ReadCommand::AuthAccount, output(11, b"", body));
        assert_eq!(result.unwrap_err().code, "CANVAS_OAUTH_SCOPE_INSUFFICIENT");
        assert_eq!(
            meta["errors"]["stderr"],
            json!({"code":"service.10000","class":"auth_required","requiredAction":"login","reason":"oauth_scope_insufficient"})
        );
        for command in [ReadCommand::ImageModels, ReadCommand::AuthStatus] {
            let bytes = classify(command, output(11, b"", body)).1.unwrap();
            assert_eq!(
                super::super::envelope(&bytes).unwrap_err().code,
                "CANVAS_READ_FAILED"
            );
        }
        let denied=br#"{"schemaVersion":"1","ok":false,"error":{"code":"service.10000","class":"access_denied","requiredAction":"none"}}"#;
        let (meta, result) = classify(ReadCommand::ImageModels, output(12, b"", denied));
        assert_eq!(meta["errors"]["stderr"]["class"], "access_denied");
        assert_eq!(
            super::super::envelope(&result.unwrap()).unwrap_err().code,
            "CANVAS_READ_FAILED"
        );
        let mut changed: Value = serde_json::from_slice(body).unwrap();
        changed["error"]["code"] = json!("OTHER_UNKNOWN_CODE");
        assert_eq!(
            super::super::envelope(
                &classify(
                    ReadCommand::AuthAccount,
                    output(11, b"", &serde_json::to_vec(&changed).unwrap())
                )
                .1
                .unwrap()
            )
            .unwrap_err()
            .code,
            "CANVAS_READ_FAILED"
        );
    }
    #[test]
    fn signal_and_output_caps_remain_distinct_without_raw_output() {
        let mut process = output(0, OK, b"");
        process.success = false;
        process.exit_code = None;
        process.signal = Some(9);
        let (meta, result) = classify(ReadCommand::AuthAccount, process);
        assert_eq!(meta["termination"], "signal");
        assert_eq!(meta["signal"], 9);
        assert_eq!(result.unwrap_err().code, "CANVAS_PROCESS_SIGNAL");
        let huge = vec![b'x'; 65537];
        let (meta, result) = classify(ReadCommand::AuthAccount, output(11, b"", &huge));
        assert_eq!(meta["stderrBytes"], 65537);
        assert_eq!(meta["stderrShape"], "not_parsed");
        assert_eq!(result.unwrap_err().code, "CANVAS_OUTPUT_LIMIT");
    }
}
