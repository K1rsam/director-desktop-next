# 官方 Codex SDK 薄桥：离线原型与发布门禁

日期：2026-10-06。当前结论：真实官方 SDK 包的离线集成已实现；产品 SDK 真实生成仍阻断，不是已完成原生验收的可用通道。

## 1. 本次实际完成的范围

- 独立 `sdk-bridge/` Node ESM 进程，实际导入并调用官方 TypeScript `@openai/codex-sdk@0.160.1`
- 精确锁定 package/lock、SDK tarball integrity、安装实现文件 SHA-256 和 Apache-2.0 声明
- 私有 NDJSON 协议 `hello / inspect / execute / observe / cancel / shutdown`
- 固定 attempt/backend/model/reasoning/原始请求/编译后 prompt/schema/hash；先持久化 dispatch intent，再调用 SDK
- 每 worker 最多一个 active attempt；unknown attempt 继续占用该 worker；不自动重试、改模型、改账号、切 CLI 或直接 API
- 独立合成 HOME、CODEX_HOME、工作目录和显式替代环境；用真实 SDK 执行自带合成子进程，验证 SDK 的实际参数、事件和错误行为
- 原始文本、JSON parse error、SDK 暴露事件和本地恢复证据分别保留；transport completion 绝不代表领域采用
- Linux 离线测试通过；无产品模型、API、用户账号或真实 Codex runtime 调用

M2 已把官方包运输接入 SavedTextExecutor、既有 jobs/raw/领域校验/恢复与 SDK 设置 UI。具体接线、持久身份、跨语言端到端证据及剩余原生门禁见 [SDK_CORE_INTEGRATION.md](SDK_CORE_INTEGRATION.md)。创作原件没有改写，SDK live 仍关闭。

## 2. 为什么选定 TypeScript SDK

先核对固定版本的实际发布包，再决定语言。本轮使用 Node ESM 调用官方 TypeScript SDK，以减少额外编译工具链；并没有把 Rust 自写 exec wrapper 改名成 SDK。

固定包事实来自 `@openai/codex-sdk@0.160.1` 的 `package.json`、`dist/index.js` 和 `dist/index.d.ts`：

| 项目 | 已核实事实 |
| --- | --- |
| Node 要求 | 官方 package engines 是 `>=18`；本轮实际 Node 为 `v24.19.0` |
| 依赖 | 精确依赖 `@openai/codex@0.160.1` |
| 实际 transport | `CodexExec.run` 启动 `exec --experimental-json`；不是 app-server |
| 真正使用的 API | `new Codex({codexPathOverride, env, config})`、`startThread(...)`、`thread.runStreamed(prompt,{outputSchema,signal})` |
| 结构化输出 | SDK 写临时 schema 文件，传 `--output-schema`，结束后清理 |
| model/reasoning | `ThreadOptions.model` 和 `modelReasoningEffort` 原样映射到对应 CLI 参数 |
| 环境 | 指定 `CodexOptions.env` 时不继承 `process.env` |
| 事件 | `thread.started` 提供 `thread_id`；固定版本的 turn 事件没有 turn ID |
| 恢复 | `resumeThread(...).run(...)` 是新生成；不是 passive observe，桥中完全没有调用 |
| 取消 | 传 Node AbortSignal，SDK 只持有直接 child；不提供进程树停止证明 |
| 字节证据 | SDK 先 JSON.parse 再 yield；部分 usage 字段会被 SDK补零。桥保留 SDK 暴露的事件，不称为逐字 runtime stdout |

官方文档：[Codex SDK](https://learn.chatgpt.com/docs/codex-sdk)。本轮可复现依据优先采用 lock 中精确的 npm 发布物，不能用会更新的文档页面推断固定版本具备额外方法。

## 3. 安装与运行

在新工程目录中：

```sh
cd sdk-bridge
npm ci --ignore-scripts --omit=optional --no-audit --no-fund
npm run verify
node src/main.mjs --state-dir /absolute/dedicated/worker-directory
```

本轮实际依赖安装使用 `npm install --ignore-scripts --omit=optional --no-audit --no-fund --cache /workspace/shared/director-next-toolchain/npm-cache --prefix /workspace/shared/director-desktop-next-worktree/sdk-bridge`，完整日志在 `sdk-bridge/verification/install.log`。只安装 SDK 和 runtime 的 JS wrapper 包；省略平台可选二进制且不执行安装脚本。不向用户旧工程、旧 node_modules 或登录目录写入。

正常启动只支持检查和本地观察；`execute` 返回 `SDK_LIVE_DISABLED`。不存在允许 live 的环境变量或命令行开关。`hello/inspect` 不构造 Codex runtime、不登录、不读账号，也不执行版本探针。

离线测试专用：

```sh
node src/main.mjs --state-dir /absolute/synthetic-worker --offline-fixture happy
```

`--offline-fixture` 只接受源码中列出的合成场景，不接受任意 executable 路径；M2 由测试代码生成合成 Node bootstrap，以已安装的原生 Node 可执行文件承接官方 SDK 发出的 `exec` 参数，再执行自带 `fake-codex.mjs`。这条 fixture 路径不用 shell、.cmd 或额外子进程，不修改官方 SDK 实现；受控 NODE_OPTIONS 只指向该合成 bootstrap，不继承用户环境。Windows 不再被代码无条件拒绝，共同测试也不再按 POSIX 条件跳过；Windows/macOS 仍须实际运行原生 CI，本轮没有两平台通过证据。

## 4. 协议 v1

所有请求都必须带：

```json
{
  "protocolVersion": "director-sdk-bridge/v1",
  "requestId": "request_1",
  "attemptId": null,
  "method": "hello"
}
```

- `requestId` 在连接内唯一；只接受 `^[A-Za-z0-9_-]{1,128}$`
- `attemptId` 在 execute/observe/cancel 中必须是相同安全标识；控制请求使用 null
- 每行一个 UTF-8 JSON object；支持跨 chunk 的 UTF-8，不对半行做 JSON.parse
- 未知版本、方法、重复 request ID、身份不匹配、hash 不匹配都失败，不转发
- 本协议属于 Director，不是官方 SDK wire protocol

执行请求：

```json
{
  "protocolVersion": "director-sdk-bridge/v1",
  "requestId": "request_2",
  "attemptId": "attempt_1",
  "method": "execute",
  "params": {
    "attempt": {
      "version": "text-attempt/v2",
      "attemptId": "attempt_1",
      "workflowId": "workflow_1",
      "logicalNodeId": "node_1",
      "backend": "codex_sdk",
      "request": { "stage": "...", "skill": "...", "model": "gpt-6.1-sol", "reasoning": "medium", "input": {}, "schema": {}, "ruleSnapshot": {} },
      "semanticRequestHash": "caller-frozen-semantic-hash",
      "executionFingerprint": "caller-frozen-execution-fingerprint",
      "state": "prepared",
      "threadId": null,
      "turnId": null,
      "lastSequence": 0,
      "rawResult": null
    },
    "compiledPrompt": "exact caller-compiled prompt",
    "compiledSchema": "{\"type\":\"object\"}",
    "compiledPromptSha256": "SHA-256 of exact UTF-8 compiledPrompt bytes",
    "compiledSchemaSha256": "SHA-256 of exact UTF-8 compiledSchema bytes"
  }
}
```

`attempt` 完整复制并持久化，额外字段（如 `runtimeIdentity`）原样保留。桥不重新编写提示词，也不替换 Rust 的 semantic hash 或 execution fingerprint 算法；调用方必须先执行 Rust `validate_identity()`。桥另存 `requestSnapshotSha256` 和完整 `frozenAttemptSha256`，恢复时重新校验它们、编译后 prompt/schema hash、原始结果 hash、journal hash 与完整 `receiptEnvelopeSha256`（包括 thread/state/usage/error/序号等元数据）。这些是内容一致性检测，不是对有权同时修改文件与 hash 的本机用户做密码学认证。schema 先核对字符串 hash，再 JSON.parse 后交官方 SDK；不声称 SDK 临时文件保留原 JSON 空白排版。

立即回复 `type:response, ok:true, result:<receipt>` 只证明本地 intent 已写入，并不证明服务端受理。随后：

- `type:sdk_event`：`attemptId/requestId/sequence/threadId/turnId:null/rawEvent/duplicate`
- `type:observation`：`state/threadId/turnId:null/lastSequence/rawResult/usage/error/domainAccepted:false`
- 失败响应：`type:response, ok:false, error:{code,message}`

最终原始结果形状：

```json
{"rawText":"unaltered SDK agent text","parsedJson":null,"parseError":"JSON parse failure, if any","domainAccepted":false}
```

合法 JSON 会写入 parsedJson；非法 JSON保留全文和 parseError，不修补标点、字段、对白或创作正文。SDK 返回失败但曾有文本时，该文本仍保存在事件日志，不能变成成功。Schema/领域验证和 publish 始终由 Rust 完成。

## 5. 持久化、恢复和容量

只写调用方提供的专用 worker 目录：

```text
worker.lock
attempt_1/
  receipt.json
  events.ndjson
  fixture-invocation.json       # 仅离线 fixture
  synthetic-runtime/           # 仅离线 fixture
```

`receipt.json` 采用同目录临时文件、文件 fsync、rename；可用时追加目录 fsync。`events.ndjson` 每条先写入并 fsync，再更新 receipt 和通知父进程。Windows 原子替换与断电耐久性仍待原生测试，不能用 Linux 通过代替。

- dispatching/running/cancel_requested 在重启后进入 submission_unknown
- 本地 observe 只读现有 receipt，不调用 SDK；找不到也不新建 attempt
- partial receipt、损坏/乱序/串身份 journal 不得被当作可重发的 prepared 状态
- identical duplicate SDK 事件保留并标记 duplicate，不二次完成；新的 terminal 后到事件、thread 变化、完成 item 改写、缺少前序事件均进入 unknown
- completed receipt 的 raw 仅能交领域层复验；不自动写 accepted/published
- 用进程锁阻止两个存活 worker 同时写一目录。取消等待超时后只要旧 task 未结束，仍保留目录锁；重启回收旧锁前先独占 `worker.lock.recovery`，重新读取 owner 并确认旧 PID 不存在，持有恢复锁直到替换 worker 锁的元数据 fsync。已有 recovery 锁直接阻断，不自动回收，避免两个重启者竞争删除新锁
- unknown 继续占用该 worker，不提供清除或强制重试命令。未来 Rust 统一预算也必须计入这些 unknown，不能靠新建 worker 绕过

限制：输入/输出单帧 1 MiB、SDK 事件 1 MiB、单 attempt journal 16 MiB/2048 条、读取 receipt 上限 4 MiB、单 worker 历史 128 attempts、输出排队 2 MiB、输入排队 32 请求、单连接总请求 2048 条。超限失败关闭，不静默截断成成功结果。错误消息限长；SDK 不暴露完整 raw runtime bytes，因此超限/非法 runtime 行的逐字完整保留不能承诺。

执行 deadline 默认 30 分钟；取消/退出给活动 task 最多 500ms 等待，之后仍为 unknown。该时间是等待预算，不是磁盘系统调用的硬实时上界，也不表示进程树已经停止。父进程关闭时本桥按有界等待退出；后代进程状态仍必须按 unknown 管理。

## 6. Rust 接线合同（M2 已接线，原生发布仍受限）

M2 的 `text_sdk.rs::OfficialSdkTransport` 与 SavedTextExecutor 已落实下述运输合同；详细实现和测试见 SDK_CORE_INTEGRATION.md。生产控制面仅状态可用，SDK live 预检仍阻断：

1. 使用受控绝对 Node/sidecar 路径和新产品专用 state directory；不得从 PATH 静默选另一个 runtime，不得传用户环境全集
2. `inspect()` 发送 hello/inspect，严格核对 protocolVersion/package/version/capabilities。`available:false` 或 `liveCallsAllowed:false` 不得变成可派发状态
3. Rust 持久化 FrozenAttempt 与 dispatch intent；以现有编译器产生 exact prompt/schema，额外冻结编译字节 hash，原样发送 execute
4. 保持一个长期拥有 stdin/stdout/stderr 和进程生命周期的受控 worker，异步读写，限制帧、队列和时间；不是每个 RPC 都随意重启 Node
5. 映射 `running + threadId` 为 `ObservationKind::Started`；`transport_completed` 映射 transport-level Completed，`raw_result` 保存整个原始结果 envelope；`submission_unknown` 映射 `ObservationKind::SubmissionUnknown`
6. 对 `rawResult.parseError` 不为空或业务 schema/validator 不通过的结果按现有拒绝/修订合同处理；不得把 `parsedJson`存在直接当采用
7. 取消不得映射成 `Stopped`，因为本实现从未返回 processTreeStopProven:true。turnId:null 是固定 SDK 的真实能力缺口
8. 对账只发送 observe。`ATTEMPT_NOT_FOUND`、丢连接、超时、损坏 receipt 都保持未知，绝不调用 resumeThread/run 或跨 CLI 重发
9. 当前 `FrozenAttempt::observe` 使用严格递增 sequence。桥 observation 的 lastSequence 是最后一条 SDK journal 序号；如果 Rust 已消费该序号的所有中间事件，则终态需要独立投影递增序号或单独 terminal receipt 应用路径。不能直接复用 lastSequence 把最终完成丢掉，也不能改写原 SDK journal 序号
10. 父进程的 SQLite/attempt 目录仍是业务事实来源。桥只保存运输证据，不直接写数据库、项目成果或媒体目录

M2 已增加 Rust SavedTextExecutor → 真 Node/官方 SDK → 合成子进程 → 原 raw/领域采用拒绝/重启缓存的跨语言 contract tests，SDK 包仍未加入 React。

## 7. 明确发布阻断项

以下不是测试已覆盖的能力，不能靠设一个启用布尔值跳过：

1. 未获得 SDK live 真实生成授权；首次原生验收仍为 CLI
2. 固定 TS SDK 没有 passive remote observe、turnId 或进程树停止证明。其能力不能满足完整 app-server 对账合同；本原型如实降级为本地证据/unknown，不假称已实现远端恢复
3. `dist/index.js` 中 `stderrChunks.push(data)` 无限制累计 stderr，readline 对单行也没有资源上限。桥层自己的帧/journal限制不能证明底层 SDK 内存有界。没有为此改写官方 SDK 或通过未经证明的进程假设绕开门禁
4. 固定 SDK 不暴露现有 CLI 隔离合同的所有启动 flags，例如 ignore-user-config/ignore-rules。当前显式 env、read-only、approval never、web disabled 是合成参数验证，不能证明真实 runtime 的配置、规则、工具、扩展及凭据加载隔离
5. 官方 runtime 在合成 HOME/断网/无凭据下的加载检查未运行；账号、实际模型、额度、真实输出质量和 latency 均未测试
6. Node/SDK/runtime/helper/许可证配套打包、干净 Windows/macOS、中文长路径完整发行物、系统进程树清理、退出和回滚均未验收
7. M2 已接入共用 Rust 执行通道并测试离线合同；它不等于真实 SDK 生成及完整双平台发行物已通过验收

如果后续正式要求 passive 对账或严格进程资源控制，需重新评估可满足条件的官方 SDK 能力与版本，或向用户说明缺口。不得自动改为自写 app-server 客户端并继续称官方 SDK。

## 8. 本轮验收证据

`npm run verify`：41 tests passed，0 failed，0 skipped，Linux x64，Node v24.19.0。完整原始日志：`sdk-bridge/verification/offline-tests.log`。`npm ci --offline --ignore-scripts --omit=optional --no-audit --no-fund --cache /workspace/shared/director-next-toolchain/npm-cache` 也已成功从已有缓存重建两项依赖，记录在 `sdk-bridge/verification/offline-ci.log`。

独立审查还复现并确认修复了 stale-lock 双持有竞争与 completed receipt threadId 被修改后误回放的问题。

覆盖实际官方包调用、exact 参数/schema/prompt、环境不继承、默认零执行、thread 身份、重复/乱序事件、malformed JSON、原始文本 parse error、单 active/unknown 占位、硬杀桥后重启、退出与取消、忽略 SIGTERM 的合成子进程、锁保留、两个 stale-lock 竞争者与遗留 recovery 锁、partial/torn receipts、六类持久内容篡改、六类元数据篡改、未知持久状态、UTF-8 分片、大输入帧和包来源固定。完整合成协议交互记录在 `sdk-bridge/verification/synthetic-protocol.ndjson`，可用 `npm run evidence` 重新生成。

实机矩阵：

| 类型 | 本轮状态 |
| --- | --- |
| Linux：实际 SDK + 合成 subprocess | 41 项通过 |
| 官方 runtime 无生成加载检查 | 未运行 |
| SDK 真实模型调用 | 未授权、未运行，调用数 0 |
| Rust↔Node 产品执行路径 | M2 已接入 SavedTextExecutor；官方 SDK 合成跨语言测试通过，生产 live 关闭 |
| macOS 原生 SDK | 未运行 |
| Windows 原生 SDK | 未运行；M2 已有 native Node 合成 launcher 路径，共同测试不跳过，等待原生 CI |

依赖证明：`sdk-bridge/verification/package-provenance.json`。日志不含用户剧本、真实环境全集、auth 文件或生产数据库。

## 9. 第三方许可与来源

- `@openai/codex-sdk@0.160.1` 和其 `@openai/codex@0.160.1` 依赖的发布 metadata 声明 Apache-2.0
- SDK tarball 的 LICENSE 文件已在官方包中核对；本次没有修改或把官方实现复制为自有 SDK
- 依赖与各平台包 integrity 保存在 package-lock.json；本轮未下载或启动平台 runtime 二进制
- `sdk-bridge/THIRD_PARTY_NOTICES.md` 记录准确包名、版本与下载来源。发行打包前仍需收集实际分发全部依赖/helper 的许可证，不能从本次元数据验证推导全部分发合规
