# Director Desktop Next

独立的 Windows 和 macOS 四阶段导演桌面项目，使用一套 Rust 业务核心、共享界面和版本化创作规则。

当前本机版本开放标准桌面的已实现渠道与第一、二阶段文字入口，保留源文、费用确认和未知提交保护；支持 App 内即梦 Canvas OAuth、账户和模型检查，额度保持未知。第二阶段美术合同、原文解析与官方SDK共享核心合成路径已经存在。最新3008的22份生产规则与旧两集采用稿已核实，但完整Stage3运行适配、生产SDK执行和Canvas生成桥仍未接入；即梦账户仍有真实授权失败记录。它不是已验收的完整生产版本。[本机修改与Windows合并说明](docs/handoff/LOCAL_STANDARD_EDITION.md)和[本机验收回执](docs/handoff/LOCAL_NATIVE_RECEIPT.json)分别记录来源、接线、实际点击结果和未验证项。

本地工程执行代理使用 GPT-6.1 Sol、Max-Max；被测桌面应用内部 CLI 使用 GPT-6.1 Sol、medium，两层配置分别核实。原 director-web 及 Mac 参考工程只读；所有新实现属于本独立仓库和另一个新目录。本轮实现CLI＋官方SDK双入口，CLI保留；首轮真实验收仅走CLI，对核验同源的第1、2集生成文本并比较旧已采用终稿。SDK本轮不live，第三阶段不生成其他集、不自动多轮全稿重跑；第一/二阶段必要的完整10集上游文字工作可做；第二阶段仅少量经核验免费的即梦官方CLI5.0图片测试；不做真实视频测试。

M3 只增加有界启动记录和单次诊断运行器，不推断或修复尚未定位的 Mac SIGABRT；[诊断范围与精确本地命令](docs/NATIVE_STARTUP_DIAGNOSTICS.md)。

M5 增加独立的 Mac 开发构建签名命令：对新产物做 ad-hoc 签名、资源封装验证，并保留扩展属性；[构建与原生复验步骤](docs/MACOS_DEVELOPMENT_BUNDLE.md)。云端合成测试通过不等于 Mac 签名或启动已经通过，原 SIGABRT 原因仍待核验。

M5.1 将扩展属性检查改为逐路径、名称和原始字节哈希比较，失败时保留脱敏证据；[证据与严格门禁](docs/MACOS_XATTR_EVIDENCE.md)。旧 M5 的签名前快照未保存，不能倒推原失败原因；新签名文件若带 provenance 等属性仍停下核验。

M6 在保留 M5.1 门禁的基础上增加独立的[即梦 Canvas 只读检查](docs/CANVAS_READINESS.md)：可选择程序，检查协议、账户和图片模型目录；与旧版即梦入口分开。额度与免费资格保持未知，尚未接入 Canvas 生成或执行真实图片任务；Mac/Windows 实际按钮与正式 CLI 兼容性待原生验收。

M6.1 仅补充[新签名节点元数据的有界验收](docs/MACOS_XATTR_EVIDENCE.md)：两个精确新节点的 provenance 必须匹配本次签名前两个目录锚点，已有属性不变，并通过严格签名、资源和最终复查。它不改系统属性或设置，也不宣称继承关系、Gatekeeper 信任或原生启动已经通过。

M6.2 补充 [Canvas 标准错误流诊断](docs/CANVAS_FAILURE_DIAGNOSTICS.md)：识别已核验的授权范围不足与模型访问被拒绝，保留脱敏退出信息。它不自动登录、重试或更改网络配置；修复后的原生提示仍需同版本复验。

M6.3 增加折叠的 [Canvas 进程阶段诊断](docs/CANVAS_PROCESS_PHASES.md)，让超时失败仍保留启动、退出、输出管道与清理的观察结果。原超时和清理规则保持；实际 Mac 原因仍待一次同版本 GUI 检查确认。

三项检查按安装、协议、账户、模型目录和真实生成分层展示，检查不提交模型请求；[连接检查合同与本地验收](docs/READINESS_CONTRACT.md)、[真正官方 SDK 的健康检查与打包](docs/SDK_READINESS.md)。

## 实施入口

1. [完整任务书](docs/IMPLEMENTATION.md)：源码证据、实现顺序、第二阶段美术拟稿与完整验收
2. [决定与真实阻塞](docs/DECISIONS.md)
3. [来源索引](docs/SOURCE_INDEX.json)：固定提交与逐文件摘要
4. [验收入口](docs/ACCEPTANCE.md)
5. [云端实现进度与运行检查](docs/IMPLEMENTATION_PROGRESS.md)
6. [官方 SDK 桥接状态与边界](docs/SDK_BRIDGE.md)
7. [SDK 共享核心接线](docs/SDK_CORE_INTEGRATION.md)与[有界原生验收](docs/NATIVE_ACCEPTANCE.md)
8. [规则版本与来源](reference-skills/README.md)：桌面初版29份、旧MacGit21份、实际Mac生产22份，原件及大小/SHA/运行角色分别保存
9. [候选创作包](rules/stage3/manifest.json)：10份原版skill＋12份支持文件，逐字节保存，inactive；实际采用来源已核对，完整运行适配仍待单独授权的源码传递
10. [Windows合并与本机修改过程](docs/handoff/LOCAL_STANDARD_EDITION.md)：已提交历史、本机修改、当前真实缺口、独立Git历史分支和平台复验步骤

整合版 SKILL 和失败试验不作为生产基线。执行前先读 AGENTS.md 与完整任务书。当前继续完整运行适配和原生验收，不重复索取整体项目授权。
