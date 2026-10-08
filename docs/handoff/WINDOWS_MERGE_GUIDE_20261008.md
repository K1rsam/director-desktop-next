# Director Desktop Next Windows 接续开发与合并指南

日期：2026-10-08。对象：Windows 端开发者及其编程助手。本文把创作来源、实际实现、已验证行为和未完成事项放在一起，帮助 Windows 端保留自己的有效更新，并与 Mac 当前成果按功能合并。本文不授权覆盖现有工作区、迁移用户数据库、强制重新登录或启动付费生成。

## 1 先明确现在可以使用什么

请以“确切提交中的代码 + 对应版本的测试回执”判断功能，不以里程碑编号、目录名或“标准版”字样判断完成度。

- 本轮已发布代码为 `05b64c920384200ce45be51ac9962c39714178cd`，分支 `handoff/mac-native-20261008`；其父提交及历史代码基线是 `2c952f67b96878a90338ab5770c7d864bcf1b78f`，即 M6.3。
- 完整历史及规则归档已经发布在 `handoff/windows-history-20261007`，提交为 `cb4f728e1f22dbf20b88e0eae0cf0a11c6c85f13`，父提交是上述 M6.3。该次提交新增 65 份资料文件，没有把 Mac 后续工作自动带入主线。
- 第二阶段美术提示词改造已在 M2 进入产品代码，包含规则、受控输入、设计有效性、最终提示词编译、队列冻结和恢复。它不是仅存了一份建议；但真实图片美观度和四视图一致性尚不能据此宣布通过。
- 最新采用的 Mac 3008 第三阶段规则已完成来源核对和保全。M6.3 的候选清单仍为 `active:false`。新提交已更新为 `production_rules_verified_runtime_adapter_pending`，仍 `active:false`，运行适配尚未完成，不能把“已上传 3008 skills”写成“最新第三阶段已完整运行”。
- Mac 后续主按钮、OAuth、账户卡及标准版入口现已随本轮提交发布。104 份变更为 24 份修改、80 份新增；发布方已逐一核对远端 104 份文件哈希。历史分支与本轮代码分支是共同基于 M6.3 的兄弟分支，历史尚未合入本轮。不能用更早未完成的 M6.4/M7 草稿覆盖这份本机实现。

### 本轮代码发布回执

本表关联已发布源码与已有本机证据。10月8日复核源码和日志指纹，没有重新构建或重跑完整测试。

| 项目 | 当前值 |
|---|---|
| 本轮 Mac 代码完整提交 SHA | [`05b64c920384200ce45be51ac9962c39714178cd`](https://github.com/K1rsam/director-desktop-next/commit/05b64c920384200ce45be51ac9962c39714178cd) |
| 本轮分支与父提交 | `handoff/mac-native-20261008`；父提交 `2c952f67b96878a90338ab5770c7d864bcf1b78f` |
| 相对 M6.3 的差异与文件清单 | [104 份变更，24 修改、80 新增](https://github.com/K1rsam/director-desktop-next/compare/2c952f67b96878a90338ab5770c7d864bcf1b78f...05b64c920384200ce45be51ac9962c39714178cd) |
| 历史分支是否已合入本轮 | 未合入；`cb4f728e` 与本轮为兄弟提交，须分别读取 |
| 72 份规则快照远端索引和哈希 | 已发布 [reference-skills/index.json](https://github.com/K1rsam/director-desktop-next/blob/05b64c920384200ce45be51ac9962c39714178cd/reference-skills/index.json)；本机来源校验通过，发布方完成远端变更文件哈希核对 |
| 已验应用版本及主程序 SHA256 | macOS arm64 0.1.0；`ea5521bafc69e0bb7306fe0bfe6391947aa61ee687296c062d1cfe22f3f1af3d`；属于10月7日产物，未宣称可复现构建证明 |
| 测试与本轮复核 | 10月7日837 core通过/0失败/24忽略，109前端、44 readiness、2标准策略、121签名属性；10月8日141源码指纹0差异、8日志哈希一致，没有全量重跑 |
| GitHub CI 状态 | 当前取得的状态及 PR触发工作流查询没有成功证据；不据此宣称CI通过或仓库绝无其他工作流 |
| 本指南版本定位 | 以 GitHub 本文件历史定位；交付消息另附文档提交链接，不在同一提交内自引用其 SHA |

固定阅读入口：[M6.3 代码树](https://github.com/K1rsam/director-desktop-next/tree/2c952f67b96878a90338ab5770c7d864bcf1b78f)；[已发布历史正文](https://github.com/K1rsam/director-desktop-next/blob/cb4f728e1f22dbf20b88e0eae0cf0a11c6c85f13/docs/handoff/WINDOWS_INTEGRATION_HISTORY.md)；[已发布历史清单](https://github.com/K1rsam/director-desktop-next/blob/cb4f728e1f22dbf20b88e0eae0cf0a11c6c85f13/docs/handoff/SKILL_ARTIFACT_MANIFEST.json)。

### Windows 必须先读的顺序

1. 先读本指南第1节，固定本轮源码和历史分支两个独立提交。
2. 读本轮 [LOCAL_STANDARD_EDITION.md](https://github.com/K1rsam/director-desktop-next/blob/05b64c920384200ce45be51ac9962c39714178cd/docs/handoff/LOCAL_STANDARD_EDITION.md)，理解实际接线；再读 [LOCAL_NATIVE_RECEIPT.json](https://github.com/K1rsam/director-desktop-next/blob/05b64c920384200ce45be51ac9962c39714178cd/docs/handoff/LOCAL_NATIVE_RECEIPT.json)，核对平台、产物、测试日期和未测范围。
3. 读 [reference-skills/README.md](https://github.com/K1rsam/director-desktop-next/blob/05b64c920384200ce45be51ac9962c39714178cd/reference-skills/README.md) 与同目录 index.json，再读 [Stage3 manifest](https://github.com/K1rsam/director-desktop-next/blob/05b64c920384200ce45be51ac9962c39714178cd/rules/stage3/manifest.json) 和 [MAC_RUNTIME_TRANSFER_REQUIREMENTS.json](https://github.com/K1rsam/director-desktop-next/blob/05b64c920384200ce45be51ac9962c39714178cd/docs/handoff/MAC_RUNTIME_TRANSFER_REQUIREMENTS.json)。后者只有静态依赖元数据，不含运行源码正文。
4. 到独立历史提交 `cb4f728e` 读 WINDOWS_INTEGRATION_HISTORY.md、SKILL_ARTIFACT_MANIFEST.json 和 reference-archive；它们不在本轮兄弟分支中，不能因当前checkout找不到就判定资料未上传。
5. 最后阅读本轮104份变更并与Windows自身HEAD逐项比较，按第9节开展合并。

### 本轮新增代码与路径对照

| 能力变化 | 主要路径 | 合并时保留的限制 |
|---|---|---|
| 官方Canvas OAuth登录、等待、取消、显式重新授权和确认退出 | `apps/desktop/src-tauri/src/canvas_oauth.rs`（新增）、`engine.rs`、`lib.rs`、`readiness_canvas.rs` | 挑战仅留进程内存，凭据由官方CLI管理；服务端scope失败仍存在，不称稳定连通 |
| 主入口与账户卡、检查刷新及折叠诊断 | `apps/desktop/src/DreaminaCanvasReadiness.tsx`、`ProviderReadiness.tsx`、`App.tsx`、`Panels.tsx`、`styles.css` | 本地登录、服务端身份、模型权限、额度分别呈现；不把旧Dreamina协议混入Canvas |
| 正常桌面标准入口与受控执行策略 | `apps/desktop/src-tauri/src/execution_policy.rs`、`native_acceptance.rs`、`main.rs`、`bin/core.rs`；前端`executionPolicy.ts`、`ExecutionPolicyBanner.tsx` | 新原生默认standard_native，已有授权/预算保留；隔离核验仍offline_prepare_only；Stage3适配、SDK live和Canvas生成桥仍受真实能力门禁 |
| 原生只读检查修复 | `apps/desktop/src-tauri/src/text_cli.rs`、`readiness_sdk.rs`、`readiness_engine_tests.rs` | Codex版本/帮助冷启动预算3秒改10秒，生成期限不变；仅macOS允许Node自身注入的`__CF_USER_TEXT_ENCODING` |
| 策略与UI回归 | `apps/desktop/src-tauri/tests/standard_execution.rs`（新增）、`m2_execution_boundaries.rs`；`ProviderReadiness.test.ts`、`executionPolicy.test.ts` | 不为测试放宽生产deadline、输出上限或未知任务保护 |
| 属性采集临时文件落点 | `scripts/macos_xattr_proof.mjs` | 使用获准目录，不改系统权限或属性；Windows不执行Mac专属签名流程 |
| 来源状态和规则追溯 | `rules/stage3/manifest.json`、`reference-skills/`、`scripts/verify_skill_lineage.py`、`docs/handoff/` | 72份参考快照非运行加载目录；当前29注册规则不被历史稿覆盖；Stage3仍inactive |

该矩阵按已发布变更路径和本机实施回执归纳。当前Stage2运行规则未被本轮历史快照覆盖；本轮也没有补齐最新3008运行器或Canvas生成桥。

## 2 为什么有三个不同的早期起点

这里按实际先后记录审查与实施，避免把建议误记为代码，也避免抹掉上游已经修好的行为。

### 5c10d31 是首次实际源码审查基线

`K1rsam/director-web@5c10d315289862e8c452cb283e6df97616af57e5` 用于形成三份主要方案：旧桌面修复逐项对照、第二阶段资产生图提示词意见、CLI 与官方 SDK 双入口实施方案。用户更早提供过 `4ea3851` 这一资料线索，但不能将未完整重建的线索版本当作实际迁入父提交。

初始目标包括：阶段格式可靠衔接、一键流程能继续推进、保留恢复能力，人物资产有可见的美术设计，第三阶段的表演、情绪和创意不因工程改造变弱。这个阶段的方案是审查结论，不是完成验收。

### 970c36f 是新增能力后的重新审查

`970c36f3ee22ed30df9981bb86f1cd703d709bd9` 是 5c10d31 的直接子提交，已有原任务恢复、视觉事实修订、主创依赖和传输保障等变化。保存的对照记录记载 34 个文件变化、增加 9503 行、删除 902 行。

旧修复候选共 20 个文件，其中 10 个也被该上游修改。直接复制旧修复会覆盖新能力，因此第二轮把建议按新版语义重写。Stage2 仍要实现人物美术及展示目标，但事实缓存、设计有效性、冻结恢复必须接入现有流程，不能另建一套互不承认的状态。

### 2ac7c21 才是 Next 的实际选择性迁入来源

`2ac7c216183f936aa08f16a93b2a24130cf5ebcd` 是 970c36f 的直接子提交，又改了 10 个文件，补充范围化历史证据、对白遗漏的整编修订、证明停止后的持久恢复及错误历史保留。Next 从这里选择性迁入 144 份源码、规则、图标和测试接口文本，详见 `docs/MIGRATED_SOURCE.json`。

因此正确顺序是：**5c10d31 审查 → 970c36f 复审 → 2ac7c21 迁入 → Next M1 及后续实施**。上游后来的 Windows 修复不是天然已合入 Next，应另外按提交与行为对照。

来源：[5c10d31](https://github.com/K1rsam/director-web/commit/5c10d315289862e8c452cb283e6df97616af57e5)、[970c36f](https://github.com/K1rsam/director-web/commit/970c36f3ee22ed30df9981bb86f1cd703d709bd9)、[2ac7c21](https://github.com/K1rsam/director-web/commit/2ac7c216183f936aa08f16a93b2a24130cf5ebcd)。

## 3 M1 到 M6.3 每一步真正解决了什么

| 阶段与提交缩写 | 修改逻辑 | 证据与限制 |
|---|---|---|
| M1 `d2ad2bd6` | 独立应用、SQLite 工作流、创建导入保存重开、受控执行与原任务身份；SDK 先有独立桥测试 | 716 Rust、81 前端、11 次合成烟测；当时 SDK 尚未进入 Rust 产品执行路径，零生成 |
| M2 `41887d1c` | 完整 Stage2 美术合同；官方 SDK 接入 SavedTextExecutor；原文解析 v6、同名重复集确认和制作范围 | 最终 775 Rust、84 前端、41 SDK 桥测试通过，24 Rust 忽略；不是实际生成质量验收 |
| M3 `09c8d58f` | Mac 启动诊断、持久启动日志和前端检查点，区分应用退出与启动等待器退出 | 5 Rust 诊断、15 隔离边界、17 运行器测试；不能凭普通开窗证明原故障修复 |
| M4 `3c6c7c30` | 将只读连接检查与生成许可分离，区分程序、协议、账户、模型、网络和真实生成 | 43 相关 Rust、93 前端、44 SDK、11 烟测；不等于全量创作回归或原生渠道闭环 |
| M5 `423b1a9f` | Mac 开发包显式构建、已知 helper 和应用临时签名、严格资源检查 | 16 包装、17 启动测试；不关闭系统安全，不等于正式签名发行包 |
| M5.1 `1b31de0d` | 原始属性字节长度与哈希证据，补目录和根节点，失败也留证据 | 25 Node、9 采集器、17 运行器；不用格式化文本比较冒充字节一致 |
| M6 `7865a6d9` | 官方 Canvas CLI 独立路径及协议检查，不套旧 Dreamina 命令 | 811 Rust、101 前端及构建；只读准备，不是 Canvas 实际出图桥 |
| M6.1 `01e9a023` | 两个新增签名节点的精确 provenance 规则，其余旧节点变化仍失败 | 121 策略对抗、29 包装、9 采集器、17 启动；本机后续构建打开成功不等于证明旧 SIGABRT 原因 |
| M6.2 `5c5d9cc9` | 有界解析 stderr 错误 JSON，区分 OAuth scope、access denied 与网络问题 | 821 Rust、24 忽略；诊断准确不等于授权自动恢复 |
| M6.3 `2c952f67` | 子进程 PID、spawn、EOF、deadline、退出和回收阶段观测 | 826 Rust、104 前端、类型检查及构建；保持超时、重试、清理策略，不是已证明的挂起根因修复 |

以上数字是相应版本的测试结果，不是可相加的独立测试总量。部分是全量，部分仅是针对模块的运行。完整提交 SHA、实施细节与证据索引见已发布历史正文和 `docs/verification/`；遇到早期局部失败记录，应同时读该阶段最终集成结果，不能只摘取一条通过或失败。

M6.4 曾尝试顶部 Canvas 路由、旧协议折叠、显式 OAuth 和刷新；M7 曾准备 Canvas 图片协议、固定提交身份、报价和未知任务保护。这些旧草稿没有因此获得产品完成状态。后续本机实现需按最终差异逐项核对，不能机械 cherry-pick 草稿，也不能把阶段号当作已完成清单。


### 每一步的问题 判断 修改与保留

下面按用户要求补齐因果过程。每段遵循“当时的问题 → 为什么这样改 → 文件或功能位置 → 保留什么 → 测试与剩余问题”。更完整的原阶段叙述请对照[历史正文第四节及第五至六节](https://github.com/K1rsam/director-desktop-next/blob/cb4f728e1f22dbf20b88e0eae0cf0a11c6c85f13/docs/handoff/WINDOWS_INTEGRATION_HISTORY.md#四-新仓库各里程碑修改逻辑)，不要把本文的压缩记述当作删除历史的依据。

1. **M1 独立工程。** 当时旧工作流有卡住、恢复不稳和反复返工的问题，直接在原工程覆盖会把数据和实验混在一起，所以先证明独立项目能创建、导入、保存、重开和保存任务证据。功能涉及共享Rust执行、SQLite与项目注册，文件来源锁在 `docs/MIGRATED_SOURCE.json`，实施依据在 `docs/IMPLEMENTATION.md`。保留旧桌面的领域校验、缓存、原回包和恢复，隔离应用标识与凭据，不接管旧数据库。716 Rust等证据仅证明受控闭环；真实模型未测，SDK此时只有独立桥测试。

2. **M2 美术与真正SDK接线。** 旧资产正文更像资料描述，缺少具体审美与展示合同；同时仅有SDK桥不代表产品能走SDK。因此规则升至 `asset-art-director` 2.0.0，加入 `pipeline/asset_design_contract.rs`，连同 `assets.rs`、`engine.rs`、`store.rs`、`workflow.rs` 和资产/制作设置UI共同处理输入、收据、编译、原子采用与冻结；官方SDK接入 `SavedTextExecutor`。保留源年龄/身份、人工稿、旧图、原任务和修订预算，不让换入口绕过未知状态。最终775 Rust、84前端、41桥测试通过，但人物图质量、SDK live及最新3008运行适配尚未验收。细节读 `docs/STAGE2_IMPLEMENTATION.md`、`docs/SDK_CORE_INTEGRATION.md`。

3. **M3 启动诊断。** Mac隔离启动曾SIGABRT且没有报告，无法区别真正应用失败与启动等待器退出，因此增加显式诊断模式、启动前持久证据和固定前端检查点。定位见 `docs/NATIVE_STARTUP_DIAGNOSTICS.md` 及相应提交；这是诊断路径，不是重写创作流程。保留普通启动与隔离执行边界，不把普通窗口开出当成隔离通过。针对性测试通过；云端缺GTK时未把该环境不能编译说成原生通过，崩溃因果仍需本机证据。

4. **M4 连接与生成解耦。** 用户发现连“测试连接”都不能使用，原因是只读检查被生成门禁一起挡住。因此给Codex、Dreamina和官方SDK增加显式readiness，区分程序、协议、账户、目录、网络和生成，读 `docs/READINESS_CONTRACT.md`。保留用户主动触发、不因进入设置页自动读账户、不登录不生成；随包SDK资源与health worker要真实存在。相关43 Rust、93前端、44 SDK通过，但不是完整创作回归或本机真实渠道闭环。

5. **M5 开发包签名。** 原始与复制应用都缺资源封印，单靠复制或修改说明不能产生可核验安装包，所以采用明确的新target构建、已知helper和应用临时签名、严格资源验证。文件入口和构建流程读 `docs/MACOS_DEVELOPMENT_BUNDLE.md`。保留系统安全设置和隔离属性，不关闭Gatekeeper，不使用正式签名秘密。16包装与17启动测试只说明开发包流程；不宣称公证或面向用户发行的Gatekeeper验收。

6. **M5.1 属性证据。** 旧属性比较依赖格式化文本，无法证明原始字节保留，于是改为有界的描述符读取、长度/SHA采集，覆盖文件、目录、根节点且失败留结构化证据。读 `docs/MACOS_XATTR_EVIDENCE.md` 及属性采集脚本。保留原有属性，不通过清除属性让构建过关。25 Node、9采集器、17运行器通过；本机发现仅新增签名节点带provenance，促成后续M6.1精确例外，未先放宽所有新增属性。

7. **M6 Canvas独立协议。** 新官方Canvas CLI和旧Dreamina命令不兼容，因此新增独立程序路径及 `readiness_canvas.rs` 对应只读通道；契约读 `docs/CANVAS_READINESS.md`。保留legacy既有生成路径，不把模型目录、登录本地状态、服务端身份和权限混为一项；余额与会员也不等于免费。811 Rust、101前端通过，证明检查合同；没有Canvas真实图片生成桥。

8. **M6.1 精确签名例外。** 已知两处新签名节点的provenance不应让正常构建永远失败，但任意放宽会失去属性保护，所以仅允许确切两个新增节点、唯一属性、非空长度和哈希同时匹配签名前根/Contents锚点。仍读上述Mac属性文档和相应提交。旧节点任何变化、额外新增节点/属性仍拒绝，严格签名及资源核验继续。121对抗、29包装、9采集器、17启动通过，本机后来成功打开；这没有追溯证明早期SIGABRT根因。

9. **M6.2 错误分流。** 真实account/model错误JSON在stderr，旧关键词把scope不足归成网络故障，因此在Canvas检查链增加有限字段解析，拒绝互相冲突的流；读 `docs/CANVAS_FAILURE_DIAGNOSTICS.md`。保留凭据输出脱敏、不自动登录、不把失败转成通过。821 Rust通过、24忽略；错误诊断更准确，但用户仍需在真实官方流程解决权限，代码没有自动修好账户。

10. **M6.3 进程观测。** 同一CLI在终端返回、GUI偶发无输出，单个TIMEOUT错误无法说明程序是否退出，于是增加spawn、PID、字节/EOF、deadline、退出与回收观测，并折叠展示；读 `docs/CANVAS_PROCESS_PHASES.md`。保留原预算、超时选择、重试和清理策略，不靠延长等待掩盖挂起。826 Rust、104前端及构建通过；本机定位main前libxpc等待但未证明根因修复。另发现用户顶部按钮仍走legacy，这是独立的UI路由错误，下方Canvas成功不能代替顶部按钮修好。

11. **M6.4/M7草稿与本机转向。** 针对顶部入口/OAuth以及Canvas出图分别准备过草稿，但前者未完成最终后端/整体回归，后者尚缺产品注册和真实图片验收。用户随后直接推动本机修改，故保留本机工作，停止用未完成草稿覆盖。有关内容在历史第四节“尚未发布的分支工作”。这两个编号不授予任何完成状态；当前提交只按实际进入代码和回执的部分确认。

12. **本轮05b64c92真实交接。** 用户要求正常标准版和可给Windows合并的完整来源，而不是仅改“离线版”字样，因此本机补正常桌面策略、官方Canvas OAuth/账户卡、原生Codex冷启动和SDK环境检查修复，并打包72份来源规则。准确路径见本指南第1节矩阵；回执与Stage3 manifest单列真实限制。保留原文授权/预算、费用与未知任务保护；Stage3运行不就绪仍阻断，SDK live和Canvas生成桥仍关闭，不把上传原件当接线完成。10月7日同产物837 core、109前端等通过，10月8日仅复核141源码与8日志哈希；当前scope不足、源DOCX/授权同源未证、最新Stage3与真实质量/Windows原生仍未完成。该提交和历史cb4是兄弟分支，分别交付给Windows，不称主线已统一。

上述顺序也解释了为什么不能选择一个缩写概括全程：5c10d31解决“先理解当时缺什么”，970c36f解决“上游已改后哪些意见仍有效”，2ac7c21解决“实际迁入哪些字节”，M1以后解决“在独立工程怎样实施和证明”，05b64c92解决“把本机真实实现和未完缺口交到Windows手中”。各自证据不能互相替代。

## 4 Stage2 已接入的内容必须整套保留

当前可直接核实的运行规则是 `apps/desktop/rules/asset-art-director/SKILL.md` 2.0.0，SHA256：

`9193cd59b44bdc77fb413fb8c40bb6531c2cfb9377ca17fafc0749808ba6cd82`

输出协议仍是 `asset-design/v1`。升级重点是让人物描述成为具体可见的美术设计，同时保留源事实及原任务保护。

### 人物设计与最终提示词

人物应有协调的脸部比例、眉眼鼻唇组合、发型轮廓、肩腰衣长、衣料结构和主辅色。好看不等于把年龄、肤色、体型、疤痕和时代抹掉；主配角和反派都可以好看，差异不能只靠换瞳色、衣色或丑化另一人。派生形象只改变有来源依据的部分，保留基础身份。

CH 基础和派生形象默认左大幅肖像、右正侧背全身，身份、发型、衣服和当前状态一致，头脚完整、背景干净、无资料面板。GROUP、BEAST、LOC、PROP 不套 CH 四视图。显式单资产构图优先于项目 CH 默认；比例依次用资产展示比例、资产比例、项目比例，仅缺失的 CH 比例回退 16:9。不要把项目 9:16 静默改成 16:9，也不要替换用户明确但不合法的 provider 参数来掩盖错误。

`projectVisualStyle`、已确认的 `productionDirection`、`imagePresentation` 和有界 `comparisonDesigns` 是输入合同。模型写设计正文，Rust 编译器组装风格、正文、展示。UI 的“设计正文”与“最终图片提示词”分别复制；最终提示词使用实际提交同一编译函数。未保存正文不能伪装为实际已提交内容。剧情集次等元数据不整段进入正文，但源中确有“第十集”招牌时不机械删除。

### 有效性和恢复保护

`asset-design-context/v2` 绑定实际规则哈希、锁定来源哈希、当前资产与相关事实语义、合法直接来源链及设计哈希、有效风格、已确认方向、构图与画幅。事实缓存有效不等于美术设计仍有效。

其他人物仅供美术对照，不成为当前人物事实、参考图片、依赖边或后续有效性依赖。最多 8 条，每条正文最多 1600 Unicode 字符，序列化对照总计最多 6400 字符；当前人物事实和必要来源不能为了塞对照被截断。实际采用的对照要随请求冻结。

批量发布先校验整个拟发布集合，再原子写入。人工稿和历史未验证稿不被静默重写。图片队列冻结最终提示词、原正文、provider/model、比例分辨率、设置、参考关系、依赖任务、账户归属及设计上下文。恢复不能重新读当前默认值替换旧请求，不能重新随机选择人物对照。来源语义变化应明确阻止不安全的续跑，已接收或未知提交继续归属原任务，不能改走 SDK 或重新提交来绕过。

### Windows 应一起审查的文件

- `apps/desktop/src-tauri/src/pipeline/asset_design_contract.rs`
- `apps/desktop/src-tauri/src/pipeline/assets.rs`
- `apps/desktop/src-tauri/src/binding.rs`、`engine.rs`、`store.rs`、`workflow.rs`
- `apps/desktop/src/AssetPanel.tsx`、`ProductionSettings.tsx`
- `apps/desktop/src-tauri/tests/stage2_contracts.rs`
- `docs/STAGE2_IMPLEMENTATION.md` 与该版最终 `docs/verification/M2.json`

只合并 SKILL.md 会漏掉冻结、有效性和原子采用；只合并 Rust 会漏掉创作正文和 UI 实际可见的提示词。真实样本仍需检查人物吸引力、角色区分、四视图同一性及模型遵守布局情况。

## 5 Stage3 的来源和接入缺口

### 哪一份是采用基准

Mac 3008 的旧 Git HEAD 是 `2511c56a52794cf5c5ee88434afafb96c787ad4c`，但最后采用版本包含工作区变化，单看旧 HEAD 不足以确定生产基准。

固定导出位于 `director-web@00093c632c45369106c7ef79a47b684fd5e77526` 的 `skill-packs/3008-stage3/2026-10-06/`。其原版目录共 22 份，10 个技能与 12 个支持文件。顶部整合稿是整理稿，不是已验证原生产入口。后续本机已报告这 22 份与实际采用任务及当前工作区匹配，并找到旧采用 EP1/EP2 终稿；这补足来源，不自动补足新桌面运行器。

M6.3 的 `rules/stage3/manifest.json` 实际写有 `candidate_not_approved_for_runtime`、`active:false`、`activePackId:null`。本轮 `05b64c92` 已把状态更新为 `production_rules_verified_runtime_adapter_pending`，来源证明增强，但 `active:false`、`activePackId:null` 和 `creative_runtime_ready() == false` 仍保留。新派发因适配未完成而阻断。因此 Windows 必须继续区分：

1. 原件是否已经找到并逐字保全；
2. 正式加载器是否真实加载这些原件；
3. runtimeContract、Schema、读取范围、调用顺序是否兼容；
4. 导演精修是否读取完整需要的范围并真正改稿；
5. 最终采用链是否采用正确结果并保存恢复状态；
6. 指定真实剧本是否达到旧采用稿质量。

仅第 1 项成立，不能将后 5 项写成完成。

### 不能在合并中丢失的创作行为

原对白、说者、声道、因果、身份、阶段状态是事实基线。情绪、表演、构图、运镜、幻想和声音共同组织，不能为减少调用数而只留标签，或让工程包装把强表演压缩成微笑、皱眉和轻微动作。

幻想时间在生产分组之前参与设计，不在最后硬塞。生产单元上限 30 秒，通常尽量 25 至 30 秒，允许有意义的自然短尾；这不是强制每单元填满或压缩台词。每单元保留完整生产通用段，不能借“去重复”删除必要交付信息。

Stage1/2 只向当前集场提供适用的人物阶段形象、场景、道具及必要历史；不把晚期形象当当前身份，不让 Stage3 重复造已有目录。精修应诊断并局部改好弱处，保留原强项；任意精修原始回包不等于最终采用稿。精修范围漏载、幻想关联误拦、恢复状态及完成计数这些历史修复需要专项回归。

### 推荐的接入完成判据

锁定原件与必要运行源码的逐文件哈希，明确 loader、contract、schema、refinement、adoption、recovery 的映射；先合成回归，再确认实际测试文件与原文、授权及预算完全对应，方可在对应授权范围内用应用内 CLI `gpt-6.1-sol / medium` 验证第 1、2 集，与旧实际采用终稿比较。对照对白完整性、情绪落点、表演、幻想机制、摄影、时长容量、阶段资产和整编保全。

旧采用稿 EP1/EP2 的 210/125 秒及 53 条原话是历史本机核验信息，不是新 Next 运行结果。历史调用次数、token 或费用也不能记成本轮测试支出。未完成对照时，版本说明应继续写“规则已保全，最新运行适配/指定集验收未完成”。

## 6 65 份历史包与 72 份本机快照如何理解

这两个数字的口径不同，不是 65 升级成 72，也不能相减得到“新加 7 个技能”。同字节不同路径仍是不同快照记录。

### 已发布历史包是 65 个文件

历史提交 `cb4f728e` 含 60 份原字节资料和 5 份说明、索引、清单。60 份资料构成为：

| 历史目录 | 份数 | 地位 |
|---|---:|---|
| `reference-archive/mac-3008-original-20261006/` | 22 | 旧 3008 实际采用来源的原件存档，归档本身不启用 |
| `reference-archive/desktop-rules-at-M6.3/` | 29 | M6.3 桌面规则快照，含已实施 Stage2 2.0.0 与继承的旧 Stage3 |
| `reference-archive/three-step-experiment/` | 4 | 未采纳三步实验和一份不同哈希第一步副本 |
| `reference-archive/proposals-5c10d31/` | 3 | 初版审查建议 |
| `reference-archive/proposals-970c36f/` | 2 | 新版复审建议 |

历史包的原始清单和正文部分仍保留 `prepared_not_uploaded` 或“待上传”措辞，这是制作时冻结的记录。**实际历史包已发布到上述固定提交。** 保留原始档案字节与说明真实发布状态可以同时做到；不要为修正发布状态悄悄改变已归档技能的内容。原 ZIP 根 README 与 DELIVERY_MANIFEST 发布时放到 `docs/handoff/`，所以读其原包相对路径时要注意发布映射，不能据此判断缺失文件。

### 本轮已发布快照是 72 个规则文件

本机只读清单报告为 30 个 SKILL.md + 42 个辅助文件，共 552341 字节；`reference-skills/index.json` 的快照记录不含另行列出的 registeredFiles。组别如下：

| 本机快照组标识 | 技能 | 支持 | 合计 | 用途 |
|---|---:|---:|---:|---|
| `desktop-original-2ac7c216` | 11 | 18 | 29 | 实际迁入桌面原规则，保存旧版本以便比较 |
| `mac-3008-production-20261005` | 10 | 12 | 22 | 本机采用生产规则快照 |
| `mac-3008-git-2511c56` | 9 | 12 | 21 | 旧 Git 历史规则快照，不覆盖生产采用稿 |

注意同为 29 份，历史包的 M6.3 桌面规则与本机 `desktop-original-2ac7c216` 不是同一版本：前者含 Stage2 2.0.0，后者保存早期美术规则 1.3。按本机比对结果，72 份中 65 份与当时 Next 主线有同字节记录、1 份美术 1.3 与 M1 同字节、6 份是未在 Next 匹配到的旧历史技能。这 6 份为 camera-emotion-methods、episode-emotion-planner、episode-finishing-editor、fantasy-expression-methods、sound-rhythm-methods、unit-performance-director 的 SKILL.md。它们是否在其他仓库发布不在该比对结论中。

72 份全部属于规则和支持材料，不包含另行讨论的 29 份 Mac 私有运行源码。不要因名称或数字接近，把这些未交付运行源码误计为已迁入。也不包含生产数据库、剧本、采用终稿、媒体或凭据。

### 运行规则与档案必须隔离

`reference-archive/` 和 `reference-skills/` 是阅读、比对与追溯入口，不能由运行器递归扫描后自动全部加载。正式规则应由明确注册路径、版本及哈希选择。旧技能、未采纳三步稿、顶部整合稿、历史副本都不能因为“更近的修改时间”自动成为生产基准。索引列出 registeredFiles 也应核对实际加载代码，不能仅凭索引就证明生效。

## 7 Mac 后续标准版工作的准确边界

下列事实已进入本轮固定提交中的本机回执；GUI与完整测试执行日期是10月7日，10月8日完成关联源码和日志指纹复核：

- 用户实际顶部“检查即梦 CLI、账户与模型”曾仍走 legacy 路径；此前下方 Canvas 卡通过并不代表这个按钮通过。后续本地主入口/OAuth 调整必须以同一安装产物实际点击验证。
- 本地后续界面已报告账户身份、会员层级、9 个图像模型、重新登录/退出及折叠技术诊断；手动检查曾成功，首次自动检查曾超时。最近标准版回执仍有 `oauth_scope_insufficient`，所以账户连通性不能写成稳定修复。
- 本机报告六个导航入口、双 CLI 检查和重开后两个项目状态保留；本地阶段测试报告 837 项 core、109 项 UI 通过。本轮已用所选141份源码指纹和8份日志哈希复核其关联，未重新执行完整测试。
- Stage2 仍为 2.0.0；Stage3 最新运行适配明确未完成。Canvas 出图实现与免费样本、SDK 真实生成、Stage4 视频不能由账户卡或测试数字推出。
- 当前 Canvas 1.0.2 的本机核验没有取得可读余额字段；余额应显示未知或接口未提供，不能编造 0。`node quote` 表示预计任务费用，不是账户余额；会员身份和模型目录不证明本次免费。服务未来可能更新，不把当时接口状态写成永久能力限制。
- M6.3 本机曾发现进程 main 前 libxpc 路径初始化挂起；单独改变 bundle 环境变量的 A/B/A 均通过，只能说明该变量单独不足以复现。没有证据时不能写“环境变量根因已修复”。

本轮固定回执确认：141份所选源码与10月7日构建后指纹全部一致，8份验证日志哈希一致；源码指纹索引SHA256为 `8a00a91838c4514419ff5fcd2a0b521f025a4b6ea4a5d714c8e74e18ea1b9e92`。10月8日重查差异格式、来源锁及72份规则；没有重新构建、登录、运行产品任务或全量测试。141份范围包括Rust/UI/当前规则/SDK源码和指定构建脚本，不是全部仓库或可复现构建证明。私有日志正文未入Git。

已有同产物完整回归为837 core通过、0失败、24忽略，共25套、4测试线程；109前端、44只读渠道、2标准策略、121签名属性测试通过。较早受限测试环境曾有657通过、9失败、20忽略，其中7项涉及回环权限、2项进程时序fixture；最终原生运行通过，未因此放宽生产deadline。不能隐去这段修复边界，也不能把最终通过改称10月8日重新执行。

### 测试原文与授权尚未完成同源证明

本轮回执明确三项均为false：`currentUploadedDocxBytesVerified`、`existingGrantIsVerifiedUploadedDocxGrant`、`existingLocalFullTextExactEpisodeMatchProven`。当前上传DOCX没有在本机核对字节；现存授权绑定数据库UTF-8原文，不能直接视为该DOCX的授权；旧两集全文与现存本机全文也未证明精确字面匹配。因此不得仅凭相似名称、旧规则一致、历史时长或已有授权就自动运行当前文件EP1/EP2，必须先完成源文件和授权范围对应核验。

已发布 `MAC_RUNTIME_TRANSFER_REQUIREMENTS.json` 仅是当前入口的静态依赖元数据，运行源码正文未传输，历史完整现场源码也未固定。该清单不能替代真实适配实现。新产品真实文字及质量比较、真实图片及免费资格、视频、SDK live、Windows原生打包和持久化均仍未验收。

## 8 测试证据按层记录

| 层次 | 能证明什么 | 不能推出什么 |
|---|---|---|
| 源码、版本、哈希 | 文件存在、来源固定、字节一致 | 运行器实际使用、模型遵守、成片质量 |
| 合成单元和集成测试 | 受控输入下合同、状态与恢复成立 | 原生桌面全部可用、实际账户和模型成功 |
| 本机 GUI 与安装检查 | 指定 OS、路径、产物下执行过的行为 | Windows 同样通过、间歇故障永久消失 |
| 真实模型任务 | 指定输入与参数的实际结果 | 所有模型、全剧、无限恢复或全平台保证 |
| 创作比较 | 指定样本在明确维度的改善与缺点 | 只有文字测试也能证明美观、泛化质量承诺 |

Windows 验证报告应记录提交 SHA、OS、工具版本、测试命令、通过/失败/忽略数、是否真实调用和实际调用范围。Rust 测试中的 live ignored 必须继续单列。UI 账户读通、模型目录读通、网络可达、真实出图分别记，不能合并成一个“连接通过”。

可参考已有 Stage2 命令，但不要假定最新 package 脚本未变；执行前核对当前配置：

```sh
cargo test --release --manifest-path apps/desktop/src-tauri/Cargo.toml --lib --no-fail-fast
cargo test --release --manifest-path apps/desktop/src-tauri/Cargo.toml --test stage2_contracts --no-fail-fast
cd apps/desktop
npm run typecheck
npm run test:frontend
npm run build
```

SDK 桥、平台打包和 GUI 测试使用最终仓库的实际脚本。任何生成验收须遵守已批准的账户、模型、输入、集数和费用范围，不因本指南提出验收建议就自动调用。

## 9 Windows 选择性合并顺序

### 第一步 固定两端和保护已有工作

先记录 Windows 自身完整 HEAD、未提交改动、使用的运行规则、现有回归及项目存储路径，再固定本轮 Mac 最终 SHA 和历史归档 SHA。另建可回退的合并分支，保留两端工作区。不要把本文中的 M6.3 旧基线当作让 Windows 回退的指令，也不要用文件夹整体覆盖。

对每项功能建立一行决策：Windows 当前行为、Next 行为、选用/合并方案、代码位置、回归证据、未解决问题。相同文件两端都改过时按行为合并，不用“Mac 较新”或“Windows 已测”作为整文件覆盖理由。

### 第二步 先合共同状态合同

优先对齐原文解析、锁定 source hash、全剧与单集制作范围、资产身份和合法来源链、任务身份、原始回包、checkpoint、修订预算及恢复。保留已经证明的停止/完成语义；未知提交不能清空后重跑，恢复旧回包不能改写其原 snapshot。

新应用 `app.director.next` 使用独立项目注册与凭据命名空间。不要因对齐路径就自动接管旧应用数据库、credentials 或项目索引。若需要迁移，另做明确可回退方案及用户授权。

### 第三步 合并完整 Stage2

按第 4 节同时处理规则、Rust 合同、UI 预览和测试，专门回归：人工稿、历史图、已有任务不被改写；来源修改时原子拒绝；同身份去重；非 CH 不套四视图；默认值修改不污染冻结请求；未知任务不重复发送。

### 第四步 单独完成 Stage3 适配

原件保持字节冻结，工程适配处理取料、字段、顺序、采用和恢复。检查旧 desktop 包装是否仍夹带与原 3008 冲突的创作约束。保留脱敏的实际 primary/methods/runtimeContract/Schema 加载清单，让审查能看见模型实际读到什么。未完成前保留准确的能力状态，不先把 active 改成 true 掩盖缺口。

### 第五步 对齐 CLI SDK 与即梦各入口

CLI/SDK 共用 Rust 业务核心、队列、领域校验、任务身份和预算。SDK 不隐式回退 CLI，切换入口不能绕开未知任务。Canvas 与 legacy 各自执行正确协议；显式 OAuth 与生成门槛分开，认证刷新不同于强制清除重登。不得将 legacy relogin 逻辑照搬为 Canvas 的默认 force。

同一版本实际检查顶部按钮、登录后的刷新、取消、重复点击、切页重返、重开应用、断线后重连和 scope 不足。Windows 另验可执行文件查找、含空格/中文路径、子进程树清理及随包 SDK 资源。

### 第六步 有选择地带入 Windows 后续修复

历史已记录的上游候选包括：非人主角目录与精确旧回包复核 `a4e203d`，legacy 账户卡 `6bc49ff`，全剧范围与覆盖率 `3fda9a4`，npm CLI 快照身份与原回包恢复 `621e49e`，图片调度及直接父图依赖 `1e2bf93`。详细内容应回到相应 `director-web` 提交读取；这些是候选来源，不是本指南宣称已经进入 Next。

尤其要保留：源哈希参与覆盖率、旧回包唯一身份复核、accepted/unknown 不重发、参考图真实直接父级、只选叶节点时有界补缺图祖先、账户变更互斥。提交/查询/下载的本地并发槽不是官方云端额度，也不能仅凭模拟吞吐声称真实速度提高。

Windows 项目注册及 MSIX AppData 重定向修复要按实际双击客户端进程核验，开发 HOST 成功不能替代。仅“无 PackageIdentity”不证明无重定向；文件句柄路径检查也不自动保证尚不存在文件的未来写入位置。registry 和 ledger 各自原子不等于跨文件事务，跨进程锁、归档/删除和已消费记录都要保留。

### 第七步 平台层分别验收 再形成共用版本

Mac 签名、属性和 bundle 检查留在 Mac 构建适配；Windows 安装器、EXE、进程和路径处理留在 Windows 适配。两端共用创作规则、Rust 业务合同和 UI 语义，避免发展成两套难以互相迁移的创作逻辑。

先跑完整受控回归，再各自原生安装与 GUI 检查，再做已授权真实样本。每项未通过都记录影响，不以“标准版”“全链路”命名掩盖。主线合并必须有两端输入提交、冲突决策和测试回执；不要把历史资料上传当成代码合并完成。

## 10 当前缺口和后续回执

本轮代码提交、104份变更的远端哈希和72份规则发布已经完成。剩余缺口是：最新3008运行适配/精修/采用链、测试DOCX与授权的同源核验、第1/2集实际比较、Canvas账户稳定性与真实图片路径、SDK live和Windows原生验收。Stage4 视频未获得本轮完成证据。

最终更新应逐项写清：完成了什么、哪份代码证明、在哪个平台执行了哪个命令或点击、仍未完成什么。凡本轮未测就写未测，凡仅历史报告就写历史报告。不要把会员信息解释成免费承诺，不把 837/109 等旧工作区测试直接贴到一个新 SHA，不使用“无 bug”“全部稳定”“全流程通过”等超出证据的措辞。

### 本轮交接快照发布核对清单

本次交付的目标是让 Windows 能读取实际代码、规则来源和当前缺口。完整 Windows 合并验收及真实生成不是发布这一交接快照的前提；未完成的项目须明确标出，不能假称通过。

- [x] 第1节已固定代码SHA、父提交、分支及链接，安装产物和测试日期据已发布回执标明
- [x] 已用141份所选源码和8份日志指纹复核关联历史测试，明确没有10月8日全量重跑
- [x] 65份历史档案与72份规则按各自提交分别发布、核对，历史分支尚未合入本轮
- [x] Stage2、Stage3、账户及生成能力按固定回执如实填写
- [x] 归档未自动启用；当前29份注册规则保留，Stage3生产候选仍inactive
- [x] 本轮交付不含私有剧本、数据库、采用终稿、媒体、凭据或私有日志正文
- [ ] 文档发布后，在交付消息提供固定文件链接及对应文档提交

### 后续 Windows 合并验收清单

以下用于接续开发完成后的验收，不表示本轮交接已经执行这些工作。

- [ ] Windows 自身更新及两端输入提交、冲突决策已记录，旧项目和冻结任务受保护
- [ ] Stage2 实际加载路径与哈希、最终 prompt 编译器已核验
- [ ] Stage3 运行适配、精修、采用与恢复完成后再更新 active 状态，并提供实际链路证据
- [ ] 最终合并代码与两端安装产物、测试报告对应
- [ ] Rust、前端、SDK 桥、原生 GUI、已授权真实任务及质量对照各自分列
- [ ] 未通过、未测试及未获授权的范围分别记录，不因版本名称扩张完成结论

本文与 `WINDOWS_INTEGRATION_HISTORY.md` 分工明确：历史正文保留完整来源与修改过程；本指南给 Windows 当前接续路径、证据解释和合并要求。交接快照可以先发布供阅读、修改和合并；它不等于全部功能或 Windows 新版本已经验收完成。
