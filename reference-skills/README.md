# 创作规则版本与来源

这里保存用户授权上传的规则原件。目录只用于来源比对，应用不会从这里加载规则。逐文件大小、版本、SHA256、来源和运行角色见 [index.json](index.json)。

| 目录 | 文件数 | 来源与状态 |
| --- | ---: | --- |
| `desktop-original-2ac7c216` | 29 | 桌面初版 `2ac7c216183f936aa08f16a93b2a24130cf5ebcd`，按迁移来源索引逐字节复核 |
| `mac-3008-git-2511c56` | 21 | 旧 Mac Git 基线 `2511c56a52794cf5c5ee88434afafb96c787ad4c` 中存在的对应文件；此提交尚无 `director-refinement/SKILL.md` |
| `mac-3008-production-20261005` | 22 | 本机实际运行目录中的原件；与旧两集生产任务冻结的22份规则、现有候选导出逐字节一致，包括未提交的导演精修规则 |

72份快照共552341字节。它们分别保留，没有覆盖同名的初版、当前版或候选版。

当前桌面注册表仍是 `apps/desktop/src-tauri/src/pipeline/mod.rs::rule_snapshots`，编译 [apps/desktop/rules](../apps/desktop/rules) 中29份规则。`asset-art-director` 当前为2.0.0，初版快照为1.3.0。注册到二进制与实际发送模型请求是不同状态：第一、二阶段采用实现中的主规则；第三阶段旧规则仍可显示和离线回归，新派发因完整 Mac 运行适配未接入而阻断。

[rules/stage3/manifest.json](../rules/stage3/manifest.json) 中的22份候选与生产规则相同，但仍 inactive。只有规则一致不能证明读包、schema、主动整集导演精修、分组采用和现场修复已接入。没有启用导出的整合 SKILL，也没有把后续短段实验替换为生产规则。

Windows 合并先读 [本机修改与合并回执](../docs/handoff/LOCAL_STANDARD_EDITION.md)。可运行 `python3 scripts/verify_skill_lineage.py` 校验本仓库的规则原件与索引；此命令不读取用户登录目录或私人剧本。
