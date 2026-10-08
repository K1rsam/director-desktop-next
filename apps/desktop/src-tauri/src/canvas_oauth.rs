//! Native, user-initiated Canvas OAuth. The official CLI owns credentials.
//! Device references stay in the worker's memory, never in IPC, logs or project data.
use serde_json::{json, Value};
use std::{io::Read, path::{Path, PathBuf}, process::{Command, Stdio}, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}, time::{Duration, Instant}};
use crate::store::{s, Result};

pub const ACTIONS: &[&str] = &["canvas.login", "canvas.loginStatus", "canvas.openAuthorization", "canvas.cancelLogin", "canvas.logout"];
#[derive(Default)]
pub struct CanvasSessions { current: Mutex<Option<Arc<Session>>> }
struct Session {
    project: String,
    path: PathBuf,
    cancelled: AtomicBool,
    done: AtomicBool,
    state: Mutex<Value>,
}
impl Session {
    fn update(&self, state: Value) { if let Ok(mut value) = self.state.lock() { *value = state; } }
    fn snapshot(&self) -> Value { self.state.lock().map(|v| v.clone()).unwrap_or(json!({"state":"failed","message":"无法读取授权状态"})) }
}

pub fn resolve(settings: &Value, resources: Option<&Path>, home: &Path) -> Result<PathBuf> {
    let selected = s(settings, "dreaminaCanvasPath").trim();
    let mut candidates = Vec::new();
    if !selected.is_empty() { candidates.push(PathBuf::from(selected)); }
    else {
        if let Ok(bytes) = std::fs::read(home.join("canvas-cli-path.json")) {
            if let Ok(saved) = serde_json::from_slice::<Value>(&bytes) {
                if let Some(path) = saved["path"].as_str() { candidates.push(PathBuf::from(path)); }
            }
        }
        let name = if cfg!(windows) { "dreamina-canvas.exe" } else { "dreamina-canvas" };
        if let Some(root) = resources { candidates.push(root.join("tools/canvas").join(name)); }
        if let Some(user) = std::env::var_os("HOME") { candidates.push(PathBuf::from(user).join(".local/bin").join(name)); }
        if let Some(paths) = std::env::var_os("PATH") {
            candidates.extend(std::env::split_paths(&paths).filter(|p| p.is_absolute()).map(|p| p.join(name)));
        }
    }
    candidates.into_iter().find_map(|p| {
        if !p.is_absolute() { return None; }
        let p = p.canonicalize().ok()?;
        crate::readiness_canvas::executable(&p).ok().map(|_| p)
    }).ok_or("未找到即梦 Canvas CLI，请在高级设置中选择已安装程序。".into())
}

// Deliberately narrow: an official auth link must not open arbitrary websites,
// local files, custom schemes, credential-bearing authorities or nested redirects.
pub fn authorization_url(input: &str) -> Result<String> {
    fn allowed(input: &str, nested: bool) -> Result<reqwest::Url> {
        if input.len() > 8192 || input.chars().any(char::is_control) { return Err("官方授权地址无效".into()); }
        let url = reqwest::Url::parse(input).map_err(|_| "官方授权地址无效")?;
        if url.scheme() != "https" || url.host_str() != Some("jimeng.jianying.com")
            || !url.username().is_empty() || url.password().is_some() || url.port().is_some() || url.fragment().is_some()
            || !["/ai-tool/cli-auth", "/passport/open/scan_user_code/"].contains(&url.path()) {
            return Err("即梦返回了未支持的授权地址，未打开网页。".into());
        }
        for (key, value) in url.query_pairs() {
            if key == "verification_uri" {
                if nested { return Err("授权地址包含重复跳转".into()); }
                allowed(&value, true)?;
            } else if !["user_code", "cli_name"].contains(&key.as_ref()) {
                return Err("授权地址包含未支持的参数".into());
            }
        }
        Ok(url)
    }
    allowed(input, false).map(|u| u.into())
}

pub fn open_authorization(input: &str) -> Result<()> {
    let url = authorization_url(input)?;
    #[cfg(target_os="macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(target_os="windows")]
    let mut command = { use std::os::windows::process::CommandExt; let mut c=Command::new("explorer.exe"); c.creation_flags(0x08000000); c };
    #[cfg(not(any(target_os="macos",target_os="windows")))]
    let mut command = Command::new("xdg-open");
    let mut child=command.arg(url).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|_| "无法打开浏览器，请点击打开授权网页重试。")?;
    std::thread::spawn(move || { let _=child.wait(); });
    Ok(())
}

fn safe_error(body: &Value) -> String {
    let error = &body["error"];
    match error["reason"].as_str() {
        Some("oauth_scope_insufficient") => "授权范围不足，请点击重新授权并在官方网页确认权限。".into(),
        Some("credential_store_permission_denied") => "官方 CLI 无法保存登录凭据，请检查本机凭据存储权限。".into(),
        Some("network_timeout") | Some("dns_resolution_failed") => "连接即梦授权服务失败，请检查网络后重试。".into(),
        _ => {
            let code = error["code"].as_str().filter(|v| v.len()<96 && v.bytes().all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))).unwrap_or("CANVAS_AUTH_FAILED");
            format!("即梦授权未完成（{code}），请重试连接。")
        }
    }
}
fn parse_output(stdout: &[u8], stderr: &[u8], success: bool) -> Result<Value> {
    let body = serde_json::from_slice::<Value>(stdout).or_else(|_| serde_json::from_slice(stderr)).map_err(|_| "即梦 CLI 未返回有效的授权结果；登录状态尚未确认。")?;
    if body["schemaVersion"] != "1" { return Err("即梦 CLI 授权协议版本不兼容。".into()); }
    if !success || body["ok"] != true { return Err(safe_error(&body)); }
    if !body["data"].is_object() { return Err("即梦 CLI 授权结果缺少数据。".into()); }
    Ok(body["data"].clone())
}

// No shell, no raw output logging. Pipes are drained concurrently and bounded.
// Cancel/timeout kills and reaps the owned child, without logging out the account.
fn run(session: &Session, tail: &[&str], timeout: Duration) -> Result<Value> {
    if session.cancelled.load(Ordering::SeqCst) { return Err("已取消等待授权。".into()); }
    let mut command = Command::new(&session.path);
    command.args(["--format","json","--profile","default","--region","cn","--non-interactive"]).args(tail)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(target_os="windows")]
    { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let mut child = command.spawn().map_err(|_| "无法启动即梦 CLI。")?;
    let (send, receive) = std::sync::mpsc::channel();
    let stdout = child.stdout.take().ok_or("无法读取即梦输出")?;
    let stderr = child.stderr.take().ok_or("无法读取即梦诊断")?;
    for (tag, stream) in [(0,Box::new(stdout) as Box<dyn Read + Send>),(1,Box::new(stderr) as Box<dyn Read + Send>)] {
        let send=send.clone();
        std::thread::spawn(move || { let mut bytes=Vec::new(); let ok=stream.take(2*1024*1024+1).read_to_end(&mut bytes).is_ok(); let _=send.send((tag,bytes,ok)); });
    }
    drop(send);
    let deadline=Instant::now()+timeout;
    let status=loop {
        if session.cancelled.load(Ordering::SeqCst) || Instant::now()>=deadline {
            let _=child.kill(); let _=child.wait();
            return Err(if session.cancelled.load(Ordering::SeqCst) {"已取消等待授权。"} else {"等待授权超时，尚未确认登录成功；请重新连接。"}.into());
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => { let _=child.kill();let _=child.wait();return Err("无法确认即梦授权进程状态。".into()); }
        }
    };
    let mut streams=[Vec::new(),Vec::new()];
    for _ in 0..2 {
        let (tag,bytes,ok)=receive.recv_timeout(Duration::from_secs(2)).map_err(|_| "即梦授权输出读取超时")?;
        if !ok || bytes.len()>2*1024*1024 {return Err("即梦授权输出异常。".into());}
        streams[tag]=bytes;
    }
    parse_output(&streams[0], &streams[1], status.success())
}

fn check_schema(data: &Value) -> Result<()> {
    if data["schemaVersion"]!="1" || data["command"]!="dreamina-canvas" || data["edition"]!="public" || data["distribution"]!="cn" {
        return Err("即梦 CLI 认证协议不兼容".into());
    }
    let auth=data["subcommands"].as_array().and_then(|list| list.iter().find(|n|n["name"]=="auth")).ok_or("即梦 CLI 未提供认证协议")?;
    for (name, required) in [("login", &[("force","boolean")][..]),("wait", &[("device-code","string"),("timeout","duration")][..])] {
        let node=auth["subcommands"].as_array().and_then(|list| list.iter().find(|n|n["name"]==name)).ok_or("即梦 CLI 缺少认证命令")?;
        for (flag,kind) in required {
            if !node["flags"].as_array().is_some_and(|flags| flags.iter().any(|v|v["name"]==*flag && v["type"]==*kind)) {return Err("即梦 CLI 认证参数不兼容".into());}
        }
    }
    Ok(())
}
fn login_state(data: &Value) -> Result<Option<(String, Value)>> {
    if data["profile"]!="default" || data["region"]!="cn" {return Err("即梦登录配置与当前连接不一致。".into());}
    if data["state"]["loggedIn"]==true {return Ok(None);}
    let challenge=&data["challenge"];
    let device=challenge["deviceCode"].as_str().filter(|v|!v.is_empty() && v.len()<4096 && !v.chars().any(char::is_control)).ok_or("即梦没有返回有效的授权挑战。")?;
    let uri=challenge["verificationUriComplete"].as_str().filter(|s|!s.is_empty()).or_else(||challenge["verificationUri"].as_str()).ok_or("即梦没有返回官方授权地址。")?;
    let uri=authorization_url(uri)?;
    let user=challenge["userCode"].as_str().filter(|v|v.len()<=128 && !v.chars().any(char::is_control)).ok_or("即梦授权码无效。")?;
    let mut public=json!({"state":"authorizing","verificationUrl":uri,"userCode":user,"message":"请在即梦官方网页确认授权，完成后会自动连接。"});
    if let Some(expires)=challenge["expiresAt"].as_str().filter(|v|v.len()<=64) {public["expiresAt"]=json!(expires);}
    Ok(Some((device.to_owned(),public)))
}

impl CanvasSessions {
    pub fn begin(&self, project: &str, path: PathBuf, force: bool) -> Result<Value> {
        self.start(project,path,Some(force))
    }
    pub fn logout(&self, project: &str, path: PathBuf) -> Result<Value> {
        self.start(project,path,None)
    }
    fn start(&self, project: &str, path: PathBuf, login: Option<bool>) -> Result<Value> {
        let mut current=self.current.lock().map_err(|_| "授权状态不可用")?;
        if let Some(old)=current.as_ref().filter(|s|!s.done.load(Ordering::SeqCst)) {
            if old.cancelled.load(Ordering::SeqCst) {return Err("上次授权正在取消，请稍后重试。".into());}
            if login.is_some() && old.project==project && old.path==path {return Ok(old.snapshot());}
            return Err("已有即梦授权正在进行，请先完成或取消。".into());
        }
        let initial=if login.is_some(){json!({"state":"starting","message":"正在连接即梦…"})}else{json!({"state":"logging_out","message":"正在退出登录…"})};
        let session=Arc::new(Session{project:project.into(),path,cancelled:AtomicBool::new(false),done:AtomicBool::new(false),state:Mutex::new(initial.clone())});
        *current=Some(session.clone());
        std::thread::spawn(move || {
            let result=(|| -> Result<()> {
                let schema=run(&session,&["schema"],Duration::from_secs(15))?;
                check_schema(&schema)?;
                if login.is_none() {
                    let has_logout=schema["subcommands"].as_array().and_then(|nodes|nodes.iter().find(|n|n["name"]=="auth"))
                        .and_then(|auth|auth["subcommands"].as_array()).is_some_and(|nodes|nodes.iter().any(|n|n["name"]=="logout" && n["writes"]==true));
                    if !has_logout {return Err("当前即梦 CLI 未提供已适配的退出登录命令。".into());}
                    let data=run(&session,&["auth","logout"],Duration::from_secs(30))?;
                    if data["loggedOut"]!=true {return Err("即梦 CLI 未确认退出登录，请刷新账户状态。".into());}
                    return Ok(());
                }
                let args=if login==Some(true) {vec!["auth","login","--force"]} else {vec!["auth","login"]};
                let data=run(&session,&args,Duration::from_secs(30))?;
                if let Some((device,mut public))=login_state(&data)? {
                    if open_authorization(s(&public,"verificationUrl")).is_err() {public["message"]=json!("浏览器未能自动打开，请点击打开授权网页。");}
                    session.update(public);
                    let data=run(&session,&["auth","wait","--device-code",&device,"--timeout","10m"],Duration::from_secs(605))?;
                    if data["profile"]!="default" || data["region"]!="cn" || data["state"]["loggedIn"]!=true {return Err("授权尚未完成，未确认登录成功。".into());}
                }
                Ok(())
            })();
            session.update(if session.cancelled.load(Ordering::SeqCst) {
                json!({"state":"cancelled","message":"已取消本次等待，登录状态请以账户检查结果为准。"})
            } else { match result {
                Ok(()) => if login.is_some(){json!({"state":"authenticated","message":"授权已完成。"})}else{json!({"state":"logged_out","message":"已退出即梦登录。"})},
                Err(message) => json!({"state":if session.cancelled.load(Ordering::SeqCst){"cancelled"}else{"failed"},"message":message}),
            }});
            session.done.store(true,Ordering::SeqCst);
        });
        Ok(initial)
    }
    pub fn status(&self, project: &str, path: &Path) -> Result<Value> {
        let current=self.current.lock().map_err(|_| "授权状态不可用")?;
        Ok(current.as_ref().filter(|v|v.project==project && v.path==path).map(|v|v.snapshot()).unwrap_or(json!({"state":"idle"})))
    }
    pub fn cancel(&self, project: &str, path: &Path) -> Result<Value> {
        let current=self.current.lock().map_err(|_| "授权状态不可用")?;
        if let Some(session)=current.as_ref().filter(|v|v.project==project && v.path==path) {session.cancelled.store(true,Ordering::SeqCst);}
        Ok(json!({"state":"cancelled","message":"已取消本次等待，现有账户授权不会被退出。"}))
    }
    pub fn shutdown(&self) {if let Ok(current)=self.current.lock() {if let Some(session)=current.as_ref(){session.cancelled.store(true,Ordering::SeqCst);}}}
    pub fn stopped(&self) -> bool { self.current.lock().map(|c|c.as_ref().is_none_or(|v|v.done.load(Ordering::SeqCst))).unwrap_or(false) }
}
impl Drop for CanvasSessions {fn drop(&mut self){self.shutdown();}}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn official_url_rejects_host_spoofing_and_nested_redirects() {
        for bad in ["https://jimeng.jianying.com.evil.test/ai-tool/cli-auth", "http://jimeng.jianying.com/ai-tool/cli-auth", "https://u@jimeng.jianying.com/ai-tool/cli-auth", "https://jimeng.jianying.com/ai-tool/cli-auth?verification_uri=https%3A%2F%2Fevil.test%2F", "https://jimeng.jianying.com/ai-tool/cli-auth?redirect_uri=file%3A%2F%2F%2Ftmp"] {assert!(authorization_url(bad).is_err());}
        assert!(authorization_url("https://jimeng.jianying.com/ai-tool/cli-auth?cli_name=canvas_cli&verification_uri=https%3A%2F%2Fjimeng.jianying.com%2Fpassport%2Fopen%2Fscan_user_code%2F%3Fuser_code%3Dsynthetic").is_ok());
    }
    #[test]
    fn challenge_secret_never_enters_public_state_and_existing_login_is_reused() {
        let data=json!({"profile":"default","region":"cn","status":"arbitrary","challenge":{"deviceCode":"PRIVATE_SYNTHETIC","userCode":"PUBLIC_SYNTHETIC","verificationUri":"https://jimeng.jianying.com/passport/open/scan_user_code/","pollSeconds":2}});
        let (secret,public)=login_state(&data).unwrap().unwrap();assert_eq!(secret,"PRIVATE_SYNTHETIC");assert!(!public.to_string().contains("PRIVATE_SYNTHETIC"));
        assert!(login_state(&json!({"profile":"default","region":"cn","state":{"loggedIn":true}})).unwrap().is_none());
        assert!(login_state(&json!({"profile":"other","region":"cn","state":{"loggedIn":true}})).is_err());
    }
    #[test]
    fn failed_auth_does_not_echo_raw_error_secrets() {
        let data=json!({"schemaVersion":"1","ok":false,"error":{"code":"cli.login_check_failed","message":"PRIVATE_SYNTHETIC","deviceCode":"PRIVATE_SYNTHETIC"}});
        let error=parse_output(&[],&serde_json::to_vec(&data).unwrap(),false).unwrap_err();assert!(error.contains("cli.login_check_failed"));assert!(!error.contains("PRIVATE_SYNTHETIC"));
    }
    #[cfg(unix)]
    fn fixture(dir: &Path, body: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path=dir.join("synthetic-cli");
        std::fs::write(&path,format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path,std::fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    #[cfg(unix)]
    #[test]
    fn cancelled_and_timed_out_owned_children_are_reaped() {
        let dir=tempfile::tempdir().unwrap();
        let path=fixture(dir.path(),"exec /bin/sleep 30");
        let session=Arc::new(Session{project:"synthetic".into(),path,cancelled:AtomicBool::new(false),done:AtomicBool::new(false),state:Mutex::new(json!({"state":"starting"}))});
        let start=Instant::now();
        assert!(run(&session,&["schema"],Duration::from_millis(100)).unwrap_err().contains("超时"));
        assert!(start.elapsed()<Duration::from_secs(3));
        let worker=session.clone();
        let child=std::thread::spawn(move ||run(&worker,&["auth","wait"],Duration::from_secs(30)));
        std::thread::sleep(Duration::from_millis(100)); session.cancelled.store(true,Ordering::SeqCst);
        assert!(child.join().unwrap().unwrap_err().contains("取消"));
        assert!(start.elapsed()<Duration::from_secs(3));
    }
    #[cfg(unix)]
    #[test]
    fn existing_login_completes_without_browser_and_session_is_project_scoped() {
        let dir=tempfile::tempdir().unwrap();
        let schema=json!({"schemaVersion":"1","ok":true,"data":{"schemaVersion":"1","command":"dreamina-canvas","edition":"public","distribution":"cn","subcommands":[{"name":"auth","subcommands":[{"name":"login","flags":[{"name":"force","type":"boolean"}]},{"name":"wait","flags":[{"name":"device-code","type":"string"},{"name":"timeout","type":"duration"}]}]}]}});
        let login=json!({"schemaVersion":"1","ok":true,"data":{"profile":"default","region":"cn","state":{"loggedIn":true}}});
        let body=format!("case \"$8\" in\nschema) printf '%s' '{}' ;;\nauth) printf '%s' '{}' ;;\n*) exit 91 ;;\nesac",schema,login);
        let path=fixture(dir.path(),&body);
        let manager=CanvasSessions::default();
        assert_eq!(manager.begin("p1",path.clone(),false).unwrap()["state"],"starting");
        assert_eq!(manager.status("p2",&path).unwrap()["state"],"idle");
        let start=Instant::now();
        while !manager.stopped() && start.elapsed()<Duration::from_secs(3) {std::thread::sleep(Duration::from_millis(20));}
        assert!(manager.stopped());
        assert_eq!(manager.status("p1",&path).unwrap()["state"],"authenticated");
    }

    #[cfg(unix)]
    #[test]
    fn logout_requires_positive_cli_confirmation() {
        for confirmed in [true,false] {
            let dir=tempfile::tempdir().unwrap();
            let schema=json!({"schemaVersion":"1","ok":true,"data":{"schemaVersion":"1","command":"dreamina-canvas","edition":"public","distribution":"cn","subcommands":[{"name":"auth","subcommands":[{"name":"login","flags":[{"name":"force","type":"boolean"}]},{"name":"wait","flags":[{"name":"device-code","type":"string"},{"name":"timeout","type":"duration"}]},{"name":"logout","writes":true}]}]}});
            let response=json!({"schemaVersion":"1","ok":true,"data":{"loggedOut":confirmed}});
            let body=format!("case \"$8:$9\" in\nschema:) printf '%s' '{}' ;;\nauth:logout) printf '%s' '{}' ;;\n*) exit 91 ;;\nesac",schema,response);
            let path=fixture(dir.path(),&body); let manager=CanvasSessions::default();
            assert_eq!(manager.logout("p1",path.clone()).unwrap()["state"],"logging_out");
            let start=Instant::now();
            while !manager.stopped() && start.elapsed()<Duration::from_secs(3) {std::thread::sleep(Duration::from_millis(20));}
            assert!(manager.stopped());
            assert_eq!(manager.status("p1",&path).unwrap()["state"],if confirmed{"logged_out"}else{"failed"});
        }
    }

}
