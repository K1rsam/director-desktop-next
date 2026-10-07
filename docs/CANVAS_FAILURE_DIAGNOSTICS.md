# M6.2 · Canvas stderr 失败诊断

基于 M6.1 `01e9a023a7255d64e4b5dbbd68766e50582cd1d9` 的窄修复。原生检查已确认两个命令的 stdout 为空，失败 JSON 写入 stderr；旧适配器只解析 stdout，导致真实结构化错误折叠成 `CANVAS_COMMAND_FAILED`。

已观察的公共错误字段（不含真实账户值或原始错误正文）：

- `auth account`：退出码 11；`code=service.10000`、`class=auth_required`、`requiredAction=login`、`reason=oauth_scope_insufficient`
- `model list --type image`：退出码 12；`code=service.10000`、`class=access_denied`、`requiredAction=none`

此前仅凭 stderr 关键词不能确定 TLS/网络问题；本修复不据关键词、退出码或 model 错误推断登录失效，不改 SSL、代理、系统配置或凭据。

## 处理边界

固定只读命令、default profile / cn region、单步/整轮时间限制和既有共享进程监督不变。共享结果仅补充操作系统实际报告的退出信号；不重试、登录、退出、刷新授权或生成。

Canvas 在内存中解析两个有界输出流。stdout 限制沿用每命令 64 KiB / 1 MiB；stderr 在已有共享 8 MiB 上限内增加 64 KiB 解析上限。只接收完整、单个 schema-1 JSON envelope；不从混杂日志抽取片段。

- 成功仍须来自成功退出后的有效 stdout success envelope，stderr success 不代替 stdout
- stderr 的 `ok=false` envelope 可提供失败；退出 0 也不会把明确错误或空输出当成功
- 两流中不同的有效 envelope 判为冲突；相同错误的重复输出可安全保留为失败
- 精确 `auth account` 四字段组合单独报告 `CANVAS_OAUTH_SCOPE_INSUFFICIENT`；其他命令、其他 code 或模型访问被拒绝均不会触发该映射
- 既有本地日志写入受限的窄停止条件保留，不能用于推断账户或网络状态

## 安全诊断

`details.processDiagnostics[固定命令名]` 仅包含退出类型、退出码、信号、两流字节数、输出形状、结构化错误所在流以及严格枚举白名单的 `code/class/reason/requiredAction`。未知枚举值显示 `unrecognized`，不原样回显。

消息区展示这些安全字段及授权范围不足的明确说明，真实账户状态仍分层保留。本地 auth status 成功不冒充服务端账户核验成功。无原始 stdout/stderr、error.message、userId、token、私有路径、环境变量或 requestId 进入报告/日志/数据库。原始缓冲区仅在本次内存处理期间存在。

进程启动失败、监督超时或共享流超限没有完整退出证据时，保留原先固定错误码，不编造退出码或字节数。真实原生复测仍由整合后的同一发布版本完成；云端回归只运行合成可执行文件。
