# 架构审计修复记录

对应 [审计报告](2026-09-04-architecture.md)。GPUI Kit 迁移与 guidance 清理
已先独立提交为 `68364f4`，下面的修改属于后续审计修复。

## 问题闭环

| 发现 | 实施结果 | 主要证据 |
| --- | --- | --- |
| F01 半帧被响应打断 | `Outbound` 统一当前帧与游标，只在帧边界选择响应优先级 | 短写 / WouldBlock / ack 插入 / 快照合并单元测试 |
| F02 绘制驱动运行时 | `Runtime` 拥有核心、Adapter 和时钟，由可取消的执行器任务推进 | `runtime_tcp` 无窗口握手、重启和持续模拟测试；实际应用验收见下文 |
| F03 丢失小数时间 | Duration 累积并保留余量，独立限制长停顿追赶 | 60 / 120 / 144 / 1000 / 2000 Hz 等时长结果一致 |
| F04 窗口失活输入 | 订阅窗口激活；清空本地 held 状态；过滤失活及激活前排队的手柄事件 | 失焦后键盘/手柄重复状态测试；实际窗口验收见下文 |
| F05 平方切帧与无预算循环 | 单次扫描后 compact；accept/read/frame/command/write/device 分别设限 | 切帧尾部保留、短帧突发、响应容量与写预算测试 |
| F06 事件跨逻辑步 | 新步骤清空旧事件；快照同时借用当前步骤与事件，无破坏性消费 | `transition_identity` 与修正后的 observation 事件测试 |
| F07 空板重启改变 board_id | 私有 revision 按锁定格实际变化更新；重启沿用未变化棋盘的标识 | 空→空、非空→空及原有 board_id 测试 |
| F08 use_hold 重复执行 | 工作副本只应用一次 Hold，仍保留 place 失败原子性 | 显式动作与 place 的 state_hash 对照及原有原子性测试 |
| F09 设置与核心状态冲突 | 打开设置通过动作暂停；远程恢复/重启使覆盖层关闭，无第二个 tick 开关 | 实际 UiState/Runtime 组合回归测试 |
| F10 null 绕过模式校验 | 按 mode 分离命令结构，拒绝外来字段和显式 null restart | 非法 null/混合字段及原有命令验证测试 |
| F11 日志阻塞和无限增长 | 默认关闭；32 条有界异步队列；8 MiB 文件和一份备份；明确 enqueue 日志语义 | 文件轮转测试，队列使用非阻塞 try_send |

## 架构与精简

- UI 只能读取 Runtime 内的 GameState；生命周期动作和时钟有明确的所属模块。
- 棋盘的 `Cell.kind` 同时表示内容与占用，移除重复 filled 字段。
- 快照直接序列化固定大小的 cells，移除非规范 `board.kinds` 扩展。
- HUD 对比数据后更新 SharedString，避免每个 tick 重新格式化整组标签。
- 音频回调独占预分配 voice 缓冲区；统一三种输出样本格式，移除锁与 scratch；每次 callback 最多取 32 个事件。
- 实时旋转与规划复用核心 SRS 候选解析，规划邻居使用固定小数组。
- 删除 active_mask、last_action/record、requested_mode 存储、未使用按键映射和 is_lock_row。
- 默认 desktop feature 保留完整应用；关闭默认 feature 可以独立编译/测试 Runtime、核心与 Adapter。CI 增加该门禁。
- 输入逻辑测试显式不初始化设备；TCP 测试不再丢弃超时前读到的半帧，idle 测试可注入单调时间。

没有引入 ECS、通用事件总线或额外业务线程；只有明确可丢弃的诊断日志移至工作线程。
核心的公开构造能力保留给现有规则测试，UI 不持有其可变引用。
state_hash 仍是协议定义的实现内 opaque identity；文档明确它不等同于完整 RNG/config checkpoint。

## 验收

- `cargo test --offline --all-targets`：135 项通过。
- `cargo test --offline --all-targets --no-default-features`：114 项通过。
- 桌面和无默认 feature 两种模式的 Clippy（`-D warnings`）通过。
- doc tests、rustfmt 和 diff 空白检查通过。
- `cargo update --dry-run` 联网预检通过，没有更新锁文件；离线预检曾因缓存中仅有撤回版本而失败。
- `cargo audit --no-fetch --no-yanked`：当前缓存公告库没有检出漏洞；保留 6 项上游维护状态警告和 `block` 的未来 Rust 兼容性提示。
- 新切帧代码隔离基准：65,536 个短帧分为每轮 64 帧处理，16 次样本总耗时中位数约 2.3 ms。原审计基准约 130 ms；两次采样时机器负载不同，结果仅作算法改进证据，不是应用延迟保证。
真实手柄硬件行为与扬声器听感不能由纯软件回归测试替代。


### Release 与实际应用

- `cargo bundle --release` 成功；首次依赖重编译约 9 分钟，窗口刷新调整后再次增量构建成功。
- 产物：`target/release/bundle/osx/gpui-tetris.app`；在源码目录外的 `/tmp/gpui-tetris-audit-verified.app` 启动验收。
- ad-hoc 签名通过 `codesign --verify --deep --strict`；icon 元数据与资源存在，10 个 WAV 与源码资源逐一校验一致。这不是公证或 Developer ID 分发签名。
- 对最终应用运行上游 `adapter_verify.py all`，ready、claim、restart、determinism 均通过。
- 发出最小化操作后，通过独立 TCP 客户端测得约 1.005 秒推进 63 个逻辑步，且 playable 为 true；随后完整协议验证仍通过。
- 可见全屏窗口下连续键盘操作的棋盘和分数更新已验证（36→58）；普通窗口缩放、全屏往返和设置控件显示已检查。
- 设置面板打开时，远程 restart 返回成功；重新激活窗口后，AX 树确认设置控件被移除。
- 验收中普通窗口后台截图有保留旧画面的现象；全屏可见时更新正常。运行时已独立于绘制，窗口上下文任务显式刷新；尚不能仅凭自动化截图区分所有遮挡/显示调度情形，未将这部分表述为完整人工视觉验收。
- 没有进行真实手柄硬件操作和扬声器听感复核；已覆盖软件输入状态、音频混音输出和资源打包。
