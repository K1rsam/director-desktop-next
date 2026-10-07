# M4 连接检查与生成权限分离

用户反馈 M1 设置页的 CLI/即梦检查按钮被离线策略整体禁用。M4 修复这一产品入口，同时保留真实生成、登录写入和凭据保存的独立门禁。程序安装、协议握手、账户、模型目录、网络和真实模型调用分别报告；不会把任何前一层直接称为后一层成功。

## 明确点击才检查

共享前端使用 `readiness.check`，provider 精确为 `codex_cli`、`dreamina` 或 `codex_sdk`。当前未保存的对应可执行文件路径会用于这一次检查，不写回项目设置。后端拒绝其他设置字段、测试 fixture、桥路径、登录或执行参数。打开页面、切换项目或切换入口均不自动访问账户；同一检查的重复点击和旧路径/旧项目回包被隔离。

普通 Windows/macOS 桌面启动开放这三个只读检查，即使 `liveExecutionAllowed` 为 false。云端 Linux、注入离线测试通道和 `--verify-startup` 隔离探针不开放宿主账户检查。探针显式使用 `Engine::new_offline_verification`，不能借新按钮访问账户。SDK 资源目录只由 Tauri 提供，不接受 IPC 中的路径。

旧 `dreamina.status` 会轮询 OAuth 并恢复任务，所以新检查完全绕过 DreaminaSessions 与原任务恢复。没有将所有 provider/credentials 动作放行。生成、登录/退出、凭据写入、真实 SDK 调用、第三阶段 inactive 门禁均保持独立。

## 分层返回合同

每个结果是 `provider-readiness/v1`，包含 provider、检查时间、以下分层状态、消息及有限的安全详情：

- installation：程序/包是否找到与验证，版本和本机可执行文件位置
- protocol：实际 CLI RPC 或 SDK worker 协议是否完成
- authentication：authenticated / not_logged_in / unknown / not_checked，以及是否核对了账户身份；不导出账户邮箱、用户 ID 或 OAuth 材料
- catalog：本次读取的模型目录。目录不是账户资格或实际生成证明
- network：独立网络证据。本轮 CLI 返回和 SDK 本地握手不冒充真实模型请求；没有单独证明时为 not_tested
- generation：本次检查固定 enabled=false、testPerformed=false、calls=0，不隐藏付费测试文本

Codex 检查使用既有的 `--version`、`exec --help` 和只读 app-server RPC：`account/read` 明确 `refreshToken=false`，随后 `model/list` 和额度查询。账户/模型 RPC 失败时保留独立取得的 CLI 版本，账户标为“未知”，不能把 SQLite 初始化失败解释为“未登录”或所选模型不支持。没有自动降级版本、换模型或登录。检查输出分别限制 stdout 256 KiB、stderr 64 KiB；RPC 在分配完整行前限制 1 MiB，并限制累计 16 MiB/2048 帧，超限后停止自有检查进程。

即梦检查使用已适配的 `version`、命令帮助和 `user_credit`，不发起图片/视频任务或 OAuth 流程，不修改队列、能力缓存表或任务状态。模型帮助和积分均不能证明当前账户的 5.0/5.0Pro 免费资格；资格始终单独标为未核验。未找到本机官方 CLI 时如实显示缺少依赖，不能用伪造的连接成功掩盖。若 version/help 虽退出0，但明确报告日志初始化因 operation not permitted/permission denied 受限，则返回 CLI_LOCAL_LOG_WRITE_RESTRICTED 并停止后续账户检查；不把它解释为登录失效，不重定向目录或修改权限。

SDK 实际导入固定官方 `@openai/codex-sdk@0.160.1`，完成 hello/inspect/shutdown 并退出，使用新的临时目录和白名单环境。它不构造推理 runtime、不读账户、不执行模型、不调用 resumeThread。真实 Codex 推理二进制不属于这个健康检查的前提，因此 SDK 包/worker 就绪仍明确显示账户、网络与真实模型未测。具体资源哈希、许可证、打包和限制见 [SDK_READINESS.md](SDK_READINESS.md)。

## 原生验收顺序

用同一份评审后 M4 提交构建应用，并一起保留 M3 启动诊断；不重复对 M3/M4 分别跑全量构建。构建前分别在 apps/desktop 与 sdk-bridge 安装锁定依赖。Tauri 构建钩子将真实 SDK 只读健康所需资源打入应用。

1. 打开普通新应用设置页，确认三项检查可点击；进入设置或切换入口时不自动触发检查
2. Codex：检测实际安装版本，读取现有账户；若 app-server 失败，回传具体错误与本地 SQLite 初始化上下文后再诊断，不重置登录或推断模型不支持
3. 即梦：先确定官方 macOS arm64 CLI 的安装来源与文件；缺失时由已授权本地工程流程补齐官方依赖，再检查实际版本、帮助和账户。免费样本仍需另行核验，不能因积分存在就出图
4. SDK：验证打包资源、Node 与实际 worker 握手；确认账户/模型未检查、真实生成仍关闭
5. 三次检查前后对照项目任务与设置，证明没有生成、恢复、登录改写或设置写回。结果分别记录，不能合并成“三条模型连接成功”

云端只使用合成 CLI、合成账户数据和无推理 SDK 健康 worker；真实本机验证及缺失安装仍需完成。M4 的测试数量、源码摘要与原生未验证项以 `verification/M4.json` 为准。
