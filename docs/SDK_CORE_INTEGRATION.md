# M2 官方 SDK 接入共享 Rust 核心

日期：2026-10-06。范围：新的 M2 工作目录。M1 目录、既有验收清单及冻结创作原件不变。

## 状态与边界

SDK 不再是 `SavedTextExecutor` 的入口占位拒绝。已实现 `SavedTextExecutor → OfficialSdkTransport → Node sidecar → @openai/codex-sdk@0.160.1 → 合成 Codex subprocess → 原始回包 → 领域采用/拒绝 → 重启回放` 路径。正常产品仍在预检阶段返回 `SDK_LIVE_DISABLED`，创建零生成任务；只有 Rust 单元测试编译中的固定离线 fixture 配置可运行合成路径。没有 SDK live 开关，也没有 CLI/API 自动后备。

这证明共用核心接线和官方包离线行为，不证明真实模型质量、账户额度、运行时隔离、原生打包或进程树停止。

## 实际代码与控制入口

- `apps/desktop/src-tauri/src/text_sdk.rs`：官方包身份 DTO、只读状态、预检、受控 Node worker、NDJSON 监督、observe/cancel
- `apps/desktop/src-tauri/src/text_executor.rs`：SDK 分支仍属于 `SavedTextExecutor`；使用既有 Store 的 jobs/events/workflow_steps、全局文本信号量和领域回调
- `apps/desktop/src-tauri/src/text_transport.rs`：保留公共 FrozenAttempt/Observation/TextTransport；移除 SDK 未实现占位；封套读取有 4 MiB 上限
- `sdk-bridge/src/bridge.mjs`：仍实际导入并调用固定官方 SDK；公开回执增加原请求/执行/编译字节摘要供 Rust 对账
- `TextExecutionSettings.tsx`：CLI/SDK 显式选择；SDK 显示固定版本、共用核心状态、账户未读取、原生未验收、live 关闭

引擎调用：

- `textSdk.status` → `text_sdk::status(&settings).await`，返回 Value；本地只读，不调用账号、Codex runtime 或模型
- 新 SDK 工作流预检 → `text_sdk::resolve_for_workflow(&settings).await`，本轮始终 `SDK_LIVE_DISABLED`
- 设置变更不改既有 workflow 的 settings 快照；执行与回调验证原 workflow/settings 绑定
- SDK unknown 不得经 `jobs.close_unknown` 清除；没有停止证明时只能保留

## 编译与不可变身份

SDK 和 CLI 调用同一个 Rust `compile_codex_input`。它继续调用 foundation/storyboard 已有无损运输编译、原 `cli_prompt` 和 strict optional schema 逻辑；不重新概括规则或修写创作正文。SDK 输出也经同一个 foundation alias/optional 恢复函数，再进入领域校验。

SDK 身份是拒绝未知字段的 `official-sdk-identity/v1`，独立于旧 CLI 的三字段对象，包含：官方包与实现 SHA、桥版本与源 SHA、协议、Node 绝对路径/文件 SHA/版本、固定 runtime package 版本、当前 synthetic/unverified runtime 身份、权限 profile、非秘密账户状态指纹。实际包 package.json、dist/index.js 和完整桥/合成执行脚本都在启动前校验；子进程环境显式清空，不继承 NODE_OPTIONS、HOME、凭据或代理配置。离线 synthetic 子进程另行明确设置 NODE_OPTIONS，只加载由固定 fixture 源生成的 bootstrap；它把原生 Node 的入口重定向到自带假 Codex 程序，不改官方 SDK，不启动额外进程。此方案移除了旧 POSIX shell launcher 对 Windows 的无条件拒绝，但不代表 Windows/macOS 原生已通过。

每个 attempt 冻结原请求、backend、model/reasoning、semanticRequestHash、executionFingerprint、attemptEnvelopeHash；Rust 另存 exact prompt/schema 与字节摘要。桥持久回执保存同一原包与编译字节。回放重新编译并核验这些绑定，不能只凭 task ID 或 completed 字样借用另一个回包。

正常非 managed CLI 新请求若已预检取得 textCliVersion，则通过实际 request_snapshot 构造完整三字段身份。没有已知版本的历史两字段快照保持旧形状，不填造假版本。旧非 managed workflow 的两字段缓存仅在父流程设置、完整领域请求、原快照哈希、原 raw/接收事件、已采用回执与 checkpoint 全部精确相符时只读回放；未知或不一致的旧节点失败关闭，不因三字段升级漏缓存而再次生成。新 SDK 身份绝不塞入旧 CLI 对象。

## 回执、序号与 raw

`lastSequence` 在桥中始终是最后一条 SDK journal 事件的序号。Rust 投影使用 SDK 事件序号乘 2；终态 observation 使用最后 SDK 序号乘 2 加 1，避免终态与最后事件同序号被丢弃。每个 SDK 事件先进入原 job 的 events，再更新冻结 attempt；重启本地 observe 时按原 journal 序号核验并补齐尚未入库的事件，重复原事件不重复写入。

文本位于原 job 的 `text-runs/<job>/result.json`，保留 SDK agent message 的 exact UTF-8 字节；SDK 返回的 parsedJson 不直接代表采用。SQLite jobs.result 和 text_received_raw 保存经过既有运输还原、尚未领域修写的值。`Store::job` 的 UI mediaIds 投影不参与原始数据比较。

领域 accept/reject 用 SQLite 原字段、最新原始接收事件比较和事务提交；采用后的领域物化结果置于 prepared 和 workflow checkpoint，SDK 原 jobs.result 不被 normalized 值覆盖。重复采用必须等于原已验证物化值；被拒绝 raw 不能经第二次 accept 晋升。原文/编译字节/快照/哈希/回执不一致则保留原证据并失败关闭。

## 恢复与持久预算

1. 先查该冻结请求的既有 job、回包和领域状态；完整回放不启动 Node 或再次 execute
2. 原节点/backend/请求变化不创建新任务
3. 原 dispatch intent 有记录但缺少可信终态时，只向原 worker 目录发送 observe；从不调用 SDK resumeThread/run 作为恢复
4. SDK unknown 保留原 job 与最初错误，不通过新 workflow、切 CLI/API、重置修订 attempt 或关闭 unknown 绕过
5. 域级 visual fact/carrier/author repair hook 仍由 SavedTextExecutor 转发。A15 原包/修订/已采用物化上下文与 accepted Finisher→后继 Author 连续性分支已分别经实际官方 SDK 合成执行证明；CLI 专用的旧快照迁移仍只用于其原 CLI 历史。新增 SDK 修订占位核验以原源/原请求/原 raw 身份和 attempt 为锚，排除 workflow/backend，保留两次上限
6. 全局四文本槽是两入口共用；SDK worker 另有一个 active attempt 的保守限制。unknown 不释放为可再次提交的容量

固定 TypeScript SDK 不提供远端被动观察，也没有 turn ID。没有本地完整完成证据时不能推断云端是否接收/停止；这也是 SDK live 不开放的原因之一。

## 监督与未完成原生门禁

Rust worker 限制 1 MiB 单帧、2048 条 SDK 事件、16 MiB journal、1 MiB sidecar stderr（最多保留 64 KiB 原字节前缀，以 hex 写入原 job 事件）、有界 stdin 写入、控制 RPC 超时和总执行 deadline；支持分片 UTF-8，非法 UTF-8/NDJSON、半行 EOF、超大帧和序号/身份变更失败关闭。inspect 到 execute 持有同一个 worker 锁，跨进程检查/启动之间没有释放锁的窗口。取消先向活动执行的等待循环发送独立信号，不等待其管道互斥锁，再通过原 worker 请求 AbortSignal；关闭有界等待，再停止直接 Node child，始终不宣称已证明整棵树停止。

重要：Rust/桥的边界限制不能约束固定官方 SDK 内部先行累积的无界 stderr 或 readline 单行。未改写官方 SDK，不以“自己加了 NDJSON 限制”冒称底层内存有界。真实运行时配置/MCP/hooks/凭据加载隔离、Node/SDK/runtime/helper 四平台打包和许可证配套、Windows/macOS 原生路径/原子替换/进程树/退出仍是发布阻断项。

## 离线复现

```sh
npm ci --offline --ignore-scripts --omit=optional --no-audit --no-fund \
  --cache /workspace/shared/director-next-toolchain/npm-cache --prefix sdk-bridge
npm run verify --prefix sdk-bridge
cargo test --release --lib sdk_core_tests --offline
cargo test --release --lib text_sdk::tests --offline
cargo test --release --test next_isolation --offline
```

Rust 命令在 `apps/desktop/src-tauri` 执行，或传该 Cargo.toml 的 manifest-path。需要已安装 Node 与固定 SDK npm 依赖；测试不得因缺少依赖悄悄跳过。`DIRECTOR_TEST_NODE` 可给合成测试明确 Node 路径；未设置时仅测试代码从 PATH 定位 Node，并冻结其实际文件 SHA/版本。产品代码没有 PATH fallback 或合成 fixture 的用户设置入口。

分项检查点日志见 `sdk-bridge/verification/m2-*`；评审修复后的最终聚合、源码摘要和精确总数以 `docs/verification/M2.json` 为准。其中 synthetic SDK subprocess 执行次数与真实模型调用次数分别记录；所有本轮 SDK 产品模型调用、账号调用和官方 Codex runtime 执行均为 0。

## 本轮专门回归覆盖

- 14 项 Rust SavedTextExecutor 跨语言/身份用例：真实官方包调用；exact prompt/schema；领域采用、拒绝与不可变 raw；原流程重启；新 workflow 精确缓存的绑定 checkpoint；桥已完成而 Rust 未落库的恢复；断连未知占位；新流程/跨 backend 拒绝绕过；两次修订预算；原 raw/编译/身份/领域哈希篡改；非法文本 JSON；A15 与 Author 连续性 hook；非 managed CLI 实际 request_snapshot 三字段构造；Stage2 同父双资产 checkpoint 与重启零额外执行
- 4 项 Rust supervisor/身份用例：UTF-8 分片与超长/半帧；stderr flood 的有界逐字前缀；SDK 独立身份类型拒绝漂移；真实官方 SDK hang subprocess 的并发 cancel、错误 attempt 拒绝以及未知状态保留
- 官方 SDK 桥既有 41 项 JS 离线回归继续通过；M2 已去掉平台跳过条件，Windows 实跑仍待 CI
- 主库、入口隔离、前端全回归的精确结果以随附 m2 日志为准；原生和 ignored live tests 没有执行
