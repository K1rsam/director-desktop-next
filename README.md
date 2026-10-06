# Director Desktop Next

独立的 Windows 和 macOS 四阶段导演桌面项目，使用一套 Rust 业务核心、共享界面和版本化创作规则。

当前是第一版离线工程：保留四阶段界面与共享 Rust 核心，可创建独立项目、导入并重开原文、编辑本地资料；真实生成、账户接入及候选 Stage3 创作包尚未启用。它不是已验收的生产版本。用户已授权助理全权推进：云端实现与离线测试 → 本地 Codex 原生工程测试 → 问题回传 → 云端修复 → 本地复测，直到验收闭环成立。

本地工程执行代理使用 GPT-6.1 Sol、Max-Max；被测桌面应用内部 CLI 使用 GPT-6.1 Sol、medium，两层配置分别核实。原 director-web 及 Mac 参考工程只读；所有新实现属于本独立仓库和另一个新目录。本轮实现CLI＋官方SDK双入口，CLI保留；首轮真实验收仅走CLI，对核验同源的第1、2集生成文本并比较旧已采用终稿。SDK本轮不live，第三阶段不生成其他集、不自动多轮全稿重跑；第一/二阶段必要的完整10集上游文字工作可做；第二阶段仅少量经核验免费的即梦官方CLI5.0图片测试；不做真实视频测试。

## 实施入口

1. [完整任务书](docs/IMPLEMENTATION.md)：源码证据、实现顺序、第二阶段美术拟稿与完整验收
2. [决定与真实阻塞](docs/DECISIONS.md)
3. [来源索引](docs/SOURCE_INDEX.json)：固定提交与逐文件摘要
4. [验收入口](docs/ACCEPTANCE.md)
5. [云端实现进度与运行检查](docs/IMPLEMENTATION_PROGRESS.md)
6. [官方 SDK 桥接状态与边界](docs/SDK_BRIDGE.md)
7. [候选创作包](rules/stage3/manifest.json)：10份原版skill＋12份支持文件，逐字节保存，inactive，等待实际采用来源核验

整合版 SKILL 和失败试验不作为生产基线。执行前先读 AGENTS.md 与完整任务书。当前继续实际 Mac 来源核对、完整适配和原生验收，不重复索取整体项目授权。
