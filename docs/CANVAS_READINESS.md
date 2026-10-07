# M6 · Dreamina Canvas 只读就绪适配

Canvas 独立补丁最初基于 M5（423b1a9ff0154e258d8197c6be8e82a23206ed01）。当前 M6 已整合到发布的 M5.1（1b31de0d4715873d0eda7739b1666268c3f649f5），保留其 macOS 签名属性快照和全部严格门禁；19 个补丁路径的旧、新摘要已逐项核对。整合后的离线检查与完整 Rust 源码一致性记录见 `verification/M6-integration.json`。这不表示原生连接、免费资格或生成验收已经通过。

## 独立入口

新增 `dreamina_canvas` 只读检查和 `dreaminaCanvasPath` 配置。设置页提供可执行文件选择与显式检查按钮，检查使用当前未保存的路径。它不替换旧 `dreamina` provider，不修改 `dreaminaPath`、生成默认值、旧工作流、账户指纹或任务恢复路由。

旧版 Dreamina 1.4.x 与 Canvas 1.0.2 不是同一命令协议。旧检查保留 1.4.x 版本及该官方发布已观测到的 `1d0bd17-dirty` 构建标识；其他未适配 version 即停止，不再尝试 `user_credit`。Canvas 入口先核验版本与 schema；不兼容协议显示 unsupported，不把安装成功、模型目录或会员状态称作生成连通或免费。

打开设置、改路径、选文件、取消文件对话框、切换页签/项目均不访问账户。重复点击、旧路径/项目/页签的迟到回包被隔离。选择/检查文件不保存设置；保存路径仍由正常的显式保存动作负责。

## 进程与最小读取

Canvas 使用既有共享 Rust CLI 监督器：无 shell、stdin 关闭、stdout/stderr 并发排空、统一关闭计数、进程退出/清理和有界超时。固定命令枚举不可由 IPC 增添参数；IPC 只允许本入口路径，不接受命令、测试 fixture、账号、令牌或环境覆写。单步 10 秒，整轮 45 秒，清理另有既有的有界期限。共享监督器各流保留上限 8 MiB；Canvas 的 JSON 解析再限制 version/account/status 为 64 KiB，schema/models 为 1 MiB。

固定只读命令：JSON `version`、JSON `schema`，经协议核验后 JSON `auth status`、`auth account`、`model list --type image`。账户读取固定到明确展示的 default profile / cn region，并使用非交互模式。没有 OAuth、login/logout、凭据保存、远端 canvas/node 创建或编辑、quote、run、素材上传、下载、重试、恢复或真实生成。

CLI 本身可能要求其正常运行所需的本地权限；遇到已分类日志/凭据存储权限错误，只报告固定脱敏代码，不修改权限、不切换用户目录、不导出凭据。应用不读取 CLI 的 token 文件或系统钥匙串。

## 分层与隐私

- 保留已验证的安装/版本，即使后续协议、账户或目录失败
- auth status 明确未登录时，不自动登录或继续远端模型查询；读取失败为未知，不能推断未登录
- auth account 不可用与已登录是独立结果；账户成功仅在安全结果中保留身份已核验布尔值及存在的会员字段；当前界面展示身份和查询状态，不展开会员字段，不导出或持久保存 userId、有效期、存储目录、原始账户输出或 token
- 不使用旧 `user_credit` 或不存在的 member 命令；当前积分查询未适配，余额始终未知，不显示 0
- 本次模型目录界面展示规范 model ID 与 mode 名称；后台安全结果保留经类型/大小校验的动态 flags、references 和会员要求，尚未形成参数编辑器。aliases 不作为模型 ID
- 缺失字段保持缺失；false、空数组与未知不同，不补写分辨率、参考数、会员要求或套餐权利
- `model <name>` 的 `matches[].generation` 与 list/find 结构不同，不交给紧凑目录解析器
- 成功的服务端 `auth account` 才提供本检查的只读网络证据；模型目录成功只证明目录可读，账户读取失败或未适配时网络仍为 not_tested。generation 始终 enabled=false、testPerformed=false、calls=0；免费资格为 unverified，尚无 Canvas 生成桥

## 后续生成边界（未实现）

远端 `canvas create` 和 `node create/edit` 即使不带 `--run` 也会写远端。已有节点的 quote 是只读，但零报价或会员状态不构成未来免费保证。单独 `node run` 没有已核验的 credit-ceiling 参数；组合 create/edit --run 的 `--credit-ceiling 0` 会先保存节点，再可能因报价停止，不能称为零副作用检查。

将来实现时必须在任何写入前冻结恢复身份；未知提交只能查原提交 ID，不重新创建或重发。本补丁未实现这些路径，也没有以设计记录解锁生成。

## 验证范围

云端使用人工构造的最小 fixture 和本地合成可执行程序，不运行已安装的真实 Canvas/Codex/Dreamina CLI，不访问账户、会员、余额或生成服务。fixture 的数值仅用于错误形状和可选字段回归，不作为真实模型能力声明。

仍需整合后的同一提交在 Windows/macOS 原生验证：选文件、正常用户上下文、正式 CLI version/schema 实际兼容、手动只读账户/目录结果和最终应用产物。Linux 测试不等于双平台原生通过。

协议依据：官方公开安装/skill 发布目录 https://jimeng.jianying.com/canvas-cli-build/1_0_0_43/ ，以及用户本机已经完成的只读 1.0.2/schema 观察；不提交本机私人账户输出。
