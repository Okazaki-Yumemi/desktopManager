## 2026-09-06 — Round 22: v1.0.1 修复轮（用户实测反馈）

- **白屏且关不掉的同步窗口（根因：主线程建窗死锁）**：`sjtu_open_sync`
  是同步命令，在主线程调 `WebviewWindowBuilder::build` 会卡死 WebView2
  初始化——窗口框出现但永不渲染（全白）、无法交互，且 build 之后的
  日志永不打印（v1.0.0 日志佐证）。改为 `async fn`（Tauri 官方要求），
  弹窗 `focused(false)` 不抢前台，自动关窗改走 `run_on_main_thread`。
  实机验证：窗口正常渲染交我办页面，开窗/关窗/日志行齐全。
- **同步拿不到数据（根因：日历应用是跨域 iframe）**：交我办把日历嵌在
  calendar.sjtu.edu.cn 的 iframe 里，同源候选端点 404、跨域候选被 CORS
  拦。改为被动嗅探：注入脚本挂钩页面自身的 fetch/XHR，第一个形状匹配
  的日历 JSON 原样经 `sjtu_receive` 推回（端点/参数/鉴权全由页面自己
  解决）；capability 增加放行 calendar.sjtu.edu.cn。端到端验证：Cookie
  自动登录 → 嗅探捕获 → count=1 入库 → 侧边栏/周视图/议程显示 →
  1.5 s 自动关窗。
- **同步按钮永久禁用**：`syncing` 只在报错或收到报告时复位，用户未
  登录就关窗口后按钮卡死。后端在 sjtu 窗口 Destroyed 时发
  `sjtu-window-closed`，前端据此复位。
- **月视图重构（用户反馈：创建的日程看不见、太丑）**：原实现月视图
  渲染时周视图永远叠加在上方，日程只有 6 px 小圆点（毛玻璃背景下
  肉眼不可见——"不显示"的实际原因）。改为周/月互斥显示 + 带标题日程
  条（时间+标题，交大琥珀/关联绿/本地强调色）+ 今日胶囊 + 选中描边
  + "还有 N 项"。
- 门禁：cargo test 63/63、clippy 0、svelte-check 0、eslint 0；产物
  MSI 3.0 MB / NSIS 2.2 MB / zip 5.0 MB / exe 6.0 MB；
  `_release_notes.md` 已删除（发布完成清理）。DECISIONS.md 修复了
  上轮插入 D24 时误吞的标题与 D1–D7（自 a16b955~1 恢复），D24 追加
  R22 修订说明。

## 2026-09-06 — Round 21: v1.0.0 发布

- 版本号三处升级（package.json / tauri.conf.json / Cargo.toml）+ 侧栏页脚
  v1.0.0 · M12（78d6754）。
- `corepack pnpm tauri build` exit 0：MSI 3.0 MB、NSIS 2.2 MB、主程序
  6.0 MB（比 0.1.0 增长来自 SJTU 同步 + ACL 清单）；两安装包压缩为
  DesktopManager_1.0.0_x64.zip（4.9 MB）。
- Release 说明 `_release_notes.md`（亮点 / 安装 / jAccount 使用步骤 /
  诚实声明：同步活体链路未经真实账号验证）。
- tag v1.0.0 推送；`gh release create` 需要用户先 `gh auth login`
  （本机无任何 GitHub 凭据），命令已写入 STATE.md 下一步。

## 2026-09-06 — Round 20: M12 交大日程集成（用户点单）+ v1.0.0 准备

- **架构（D24）**：同步窗口打开 my.sjtu.edu.cn/ui/calendar，用户自己登录
  jAccount（凭据与 Cookie 只存在于系统 WebView，应用无任何读取路径）；
  注入脚本同源拉取日历 JSON（候选端点：/ui/api/calendar →
  /ui/api/event/list → calendar.sjtu.edu.cn/api/event/list，未登录探测
  +社区教程双线索，全同源优先），经唯一一条只读 IPC 命令 `sjtu_receive`
  推回。capability `sjtu-remote` 只放行这一条命令给远程页面；为使 ACL
  生效，新增应用权限清单——**全部 53 条应用命令显式列入
  allow-app-commands**（新命令必须同步登记，否则前端调用被拒）。
- **后端**：迁移 0009 `sjtu_events`（外部 ID 唯一、每次同步整表事务替换，
  不留脏行）；`sjtu.rs` 宽容解析（包装/裸对象、秒/纯日期格式、坏时间跳过
  计数、2 MB 上限）；`sjtu_list/sjtu_clear/sjtu_open_sync` 命令；同步成功
  发 `sjtu-synced` 事件并 1.5 s 后自动关同步窗口；主窗口关闭策略改为仅
  main 隐藏到托盘（辅助窗口正常关）。9 个新单测（含用户提供的完整样例
  JSON 断言）。cargo test 63/63、clippy 0。
- **前端**：`sjtu.svelte.ts` store + `SjtuSidebar.svelte`（正在上课/下一节
  课卡片、30 s 刷新倒计时、今日列表、同步时间/条数、清除按钮）；日历页
  周/月/日程三视图合并交大条目（amber 色带 + 「交大」标签、无删除钮）；
  课前 10 分钟提醒（toast + 双音提示音，会话内每条一次）；设置页新增
  「上海交大日程」分区（同步/两步确认清除）。svelte-check 0、eslint 0、
  vite build ok。
- **验证**：降级浏览器冒烟（侧边栏渲染、样式探针 264px/12px、两处同步
  按钮降级 toast 优雅）；真实应用短启动：迁移 v9 应用成功、ACL 清单
  无报错、7 s 后正常终止。jAccount 登录→同步的活体链路待用户验证。
- **下一步**：版本号 1.0.0 → release 构建 → zip → tag + GitHub Release
  （gh 未登录，需用户 `gh auth login`）。

## 2026-09-06 — Round 19: M11 冒烟 — release 构建 + 安装包

- `pnpm tauri build` 全绿（vite build → cargo release → 打包，exit 0、
  无警告）：MSI 2.8 MB、NSIS 2.0 MB、主程序 5.4 MB。产物在
  src-tauri/target/release/bundle/{msi,nsis}/，等用户安装验收。
- docs/PERFORMANCE.md 基线表回填二进制/安装包体积（MEASURED）；
  启动耗时/空闲 CPU/RAM 仍需真实会话测量。
- STATE.md 新增「晨间验收清单」：过夜 7 轮交付一览 + 10 分钟手工验收
  步骤（外观偏好、桌面图标大小、月视图 + ICS 导出、提示音、损坏 DB
  演练、安装包覆盖安装）。
- **过夜任务到此收口，等待统一验收。**

## 2026-09-06 — Round 18: 日程 ICS 导出（M6 遗留）

- 新增 `calendar_ics.rs`（纯序列化 + 6 个单测）：RFC 5545 转义（; , \n \）、
  75 字节折行（按字符边界，中文安全，展开可还原）、CRLF 行尾；时间采用
  浮动本地时间（应用无时区模型，不谎称 UTC/TZID，D23）；全天事件用
  VALUE=DATE + 排他 DTEND（+1 天）。
- `CalendarRepo::list_all()` + `event_export_ics` 命令：导出全部日程到
  应用数据目录 exports/calendar-<时间戳>.ics，返回 {path, count}；
  日历页新增「导出 ICS」按钮，成功提示路径，降级模式失败提示优雅。
- 测试：cargo test 54/54、clippy 0、svelte-check 0、eslint 0；降级浏览器
  冒烟验证按钮渲染与失败路径。真实应用内导出 + 第三方日历互导留给用户
  会话（TESTED，未标 WINDOWS_TESTED/USER_VERIFIED）。
- **下一步：** M11 release 构建冒烟（pnpm tauri build + 安装包产物）；
  汇总晨间验收清单。

## 2026-09-06 — Round 17: M9 性能测量切片（0/50/200/500 项）

- 新增 `src-tauri/src/perf_measure.rs` 测量基建（非 CI：
  `cargo test m9_ -- --ignored --nocapture`，best-of-N，debug 构建取
  上界）；结果转录进 docs/PERFORMANCE.md（MEASURED）。
- 结论：500 项时扫描 0.54 ms、首次同步 9.01 ms、重复同步
  6.74 ms、列表 0.73 ms、搜索 0.19 ms、含 D22 quick_check 的打开
  1.84–2.84 ms —— 全部 ≤ 10 ms，按章程“只优化测量证明有必
  要的”，本轮不做任何优化。
- 未测（需实机会话）：启动到可用、空闲 CPU/RAM（含
  WebView2 全家桶）、图标提取、fs 事件→UI 延迟、安装包体积。
- **下一步：** 继续铺遗留功能（ICS 导出、布局预设、
  性能模式）；之后 M10/M11（release 构建 + 安装包）。

## 2026-09-06 — Round 16: M8 离线可靠性切片（损坏 DB 隔离恢复 + 规模测试）

- `storage::Database::open_with_recovery`（M8 P0）：检测两类损坏
  （打开即 SQLITE_NOTADB/CORRUPT；`PRAGMA quick_check` ≠ ok），把 db
  与 -wal/-shm 兄弟文件改名隔离为 *.corrupt-<epoch-ms>（绝不删除，
  用户可手工抢救），随后全新建库继续启动；非损坏类错误
  维持响亮失败。AppState::init 已接入，隔离事件写入日志（D22）。
- 规模测试：临时目录 500 文件 + 20 文件夹 → 扫描 0 ms /
  首次入库 5 ms（debug 构建）；二次同步零变更；删除 250 个
  文件后 soft-remove（missing=1）收敛，可见计数 270。
- 测试：cargo test 48/48（新增 4 个）、clippy 0。真实应用的
  损坏演练（故意写坏线上 DB）留给用户会话演示。
- **下一步：** M9 性能测量（docs/PERFORMANCE.md，0/50/200/500 项）。

## 2026-09-06 — Round 15: 桌面图标大小设置（M7 遗留收尾）

- 设置页「外观」新增 图标大小 行（小/中/大，ui.iconSize，走 enumPref 工厂 →
  data-icon-size）；tokens.css 定义 --desktop-cell/--desktop-icon 三档
  （185/230/290px 单元宽与 28/36/44px 图标边长），DesktopPage 网格与图标
  改为消费变量，自动填充列数随档位自然重排。
- 冒烟发现真 bug：Chromium 的 dataset setter 拒绝带连字符的属性名
  （dataset["icon-size"] 抛 SyntaxError，且抛在内存赋值之后，UI 呈半生效态）；
  enumPref 改为 setAttribute("data-…")（D21）。早前四偏好属性名无连字符，
  未受影响。
- 降级浏览器冒烟：三档切换后 data-icon-size 与计算值全部正确
  （185/28、230/36、290/44）；作用域探针元素证实桌面网格 minmax 重排
  （大档 4→3 列、图标 28/36/44px）；重载后 load() 正常落 attribute。
  svelte-check 0 / eslint 0 / vitest 6。
- **下一步：** M8 离线可靠性切片（损坏 DB 恢复 + 数百文件规模测试）。

## 2026-09-06 — Round 14: 专注完成提示音 + 自定义强调色

- chime.svelte.ts：WebAudio 双音提示（专注结束 E5→A5 上行，休息结束下行），
  无音频资源；自动播放策略下 resume 失败静默放弃，绝不打断专注流程；
  ui.sound 布尔偏好持久化，设置页新增「通知」区开关（手动结束不响）。
- 自定义强调色（M7 遗留）：强调色行尾新增取色器 swatch；选中色以内联
  CSS 变量覆写（--accent + color-mix 派生 --accent-soft），稳赢所有
  [data-accent] 预设块；切换回预设时清除内联值；ui.accentCustom 持久化，
  非法值由 HEX 白名单拒绝。
- App.svelte 启动加载两个新偏好。svelte-check 0 / eslint 0 / vitest 6；
  降级浏览器冒烟验证取色双向切换与提示音开关状态。
- **下一步：** 桌面图标大小设置（M7 遗留），随后 M8 离线可靠性切片。

## 2026-09-06 — Round 13: M6 补全（日历月视图）

- CalendarPage 新增月视图：42 格周一起始网格，逐日事件圆点（关联任务绿色），
  超出 3 个显示 +N；非当月日期置灰；点选日期即同步下方当日 Agenda，
  双击打开全天新建表单；周/月切换持久化到 ui.calendarView。
- 事件拉取范围随视图切换（周：当前周；月：6 周覆盖首尾）。
- 测试：svelte-check 0 / eslint 0 / vitest 6；降级浏览器冒烟验证
  42 格、选日联动、跨月置灰与往返翻月、切回周视图正常。
- **下一步：** 继续铺遗留功能（专注完成提示音 / 桌面图标大小 / 自定义
  强调色 / ICS 导出），随后进入 M8 可靠性离线切片。

## 2026-09-05 — Round 12: M7 个性化核心（外观四偏好）

- 新增四个枚举外观偏好：外观风格（标准/柔和/硬朗/纯黑）、密度（舒适/紧凑）、
  毛玻璃强度（关闭/轻/标准/强）、动效（标准/减弱/关闭）。共用一个 enumPref
  工厂：settings 表持久化 + `<html data-*>` 镜像，tokens.css 独自承担视觉
  后果（D20）。纯色 OLED 只改深色调色板（浅色主题下诚实地不生效）；
  动效关闭同时全局禁用 transition/keyframes。
- tokens.css 重构：--glass 改为吃 --glass-alpha、--glass-filter 吃
  --glass-blur，各档位只调两个变量；紧凑密度缩放间距与字号档。
- 设置页外观区新增四行分段控件；App.svelte 启动时统一加载。
- 测试：svelte-check 0/eslint 0/vitest 6（新增 resolveEnum 白名单测试）；
  降级浏览器冒烟确认四控件实时改计算样式（背景纯黑、间距 5px、blur 34px、
  时长 0ms）。应用内可视化验收留给用户。
- 踩坑记录：锁屏隐藏标签页下 vite HMR 会给带时间戳的模块 URL 提供过期
  变换，表现为应用不挂载/样式不更新；touch 源文件 + 整页重载可解。
- **下一步：** M8 — 可靠性（重启/损坏 DB/缺文件等对抗性场景），或按
  ROADMAP 顺序继续铺功能。

## 2026-09-05 — Round 11: M6 任务 + 日历核心

- 后端：tasks（todo/doing/done、优先级 0–3、截止日、预计分钟、标签去重、备注）
  与 calendar_events（起止 epoch ms、全天、关联任务外键、重排）两个仓储 +
  10 个 Tauri 命令；任务状态唯一入口 set_status——完成盖 completed_at、
  退出即清；日历范围查询用 [from, to) 重叠语义，全天事件优先排序。
- 前端：任务页（Ctrl+N 快速捕获、状态点按循环、优先级/截止/备注就地编辑、
  状态过滤 + 搜索）；日历页（周一起始周视图、点小时槽建日程、全天/timed
  两类事件、关联任务绿色边条、当日 Agenda、删除）；专注页新增绑定任务下拉，
  会话落 taskId。
- 测试：44 单测（新增 6）+ clippy 0 + svelte-check 0 + eslint 0——顺手修掉
  4 处 eslint Date 变异报错（改非变异构造，页面日期工具全部无副作用）。
- 冒烟：夜间锁屏导致 WebView a11y 不可用，改用 vite dev（无 Tauri 后端的
  降级模式）浏览器渲染验证：两页结构完整、后端错误走 toast 不崩溃、周区间
  8月31日–9月6日 与槽位预填 2026-09-05T09:00 正确、表单取消正常。
  应用内可视化验收留给用户早上统一过。
- **下一步：** M7 — 个性化（主题预设 / 强调色 / 不透明度 / 密度）。

## 2026-09-05 — Round 10: 用户体验回合（毛玻璃 / 重命名 / 子集合 / 欢迎页）

- 用户四条反馈全部落地：① 主要表面改半透明毛玻璃（tokens --glass +
  --glass-filter，侧栏/卡片/芯片/条目/搜索/吐司全覆盖，D18）；
  ② 集合芯片上的铅笔按钮就地重命名（后端 collection_rename 已有，前端补齐，
  顺带加了重名查重）；③ 子集合：迁移 0008 加 parent_id，芯片行按父链缩进
  渲染，活动集合上 FolderPlus 建子集合，删除级联整棵子树，层级上限 5（D19）；
  ④ 文件夹引用可在应用内展开：browse_children 只读列目录（上限 500、隐藏项
  跳过、目录优先），打开/图标/浏览白名单延伸到“集合所持或桌面索引目录的
  子项”，面包屑导航可逐级返回；⑤ 今天页改为欢迎界面（大时钟 + 每日一句 +
  今日专注统计），后端状态与快速上手移入设置页，侧栏徽标更新为 M5。
- 测试：39 单测（新增嵌套/深度上限/子树级联/目录列表 4 个）、clippy 0、
  svelte-check 0、eslint 0；真机确认欢迎页渲染与 0008 迁移落库。
  交互项（重命名/展开）等用户实际点击验证。
- **下一步：** M6 — 任务 + 日历（todo/doing/done、快速捕获 Ctrl+N、Agenda/周视图）。

## 2026-09-05 — Round 9: M5 专注核心（数据库即时钟）

- focus_sessions（0005 表首次启用）：focus_repo start（运行中拒绝再开、
  task/scene id 存在性校验、count_up 计划时长记 0）/ running 恢复 / finish
  （completed|abandoned，实际时长取墙钟差）/ 打断计数 / 笔记 / 按日列表 /
  近 N 天汇总（SQLite localtime 分桶，本地日期串由前端计算）。
- FocusPage：预设 25/5、50/10、自定义分钟、正计时；大倒计时 + 进度条 +
  超时状态；打断/完成/放弃；备注失焦即存；场景绑定下拉 + 一键应用场景
  （soft，写 ui.activeScene，不强切）；今日记录 + 近 7 天条形汇总；预设
  持久化 focus.preset。恢复语义：页面挂载读 running 行，重启不丢计时；
  页面打开时到点自动完成并进入休息，恢复的超时会话绝不自动判定（D17）。
- eslint 新规则：$state 包裹 SvelteSet 报错——DesktopPage sceneHidden 改为
  纯 SvelteSet 变更（clear/add/delete），FocusPage 全程遵守。
- 验证：cargo test 36/36（另 2 个 ignored 真机用例）、clippy 0、
  svelte-check 0、eslint 0；真机冒烟：开始→（用户）备注→完成 00:34→
  toast+自动休息→跳过休息→待开始；用户自测了一条 00:02 放弃记录；
  node:sqlite 确认 2 行落库 + focus.preset 持久化。
- 同日用户确认 M3/M4 两项手动测试通过：外部快捷方式拖入集合、设置页
  布局保存→应用。M3 布局恢复至此 USER_VERIFIED。
- 遗留：托盘计时状态、迷你计时窗、结束提示音、任务绑定 UI（M6 后补）。

**下一步：** M6 — 任务 + 日历（todo/doing/done、快速捕获 Ctrl+N）。


## 2026-09-05 — Round 8: M4 场景核心 + 白屏修复

- 白屏根因：dev 重启竞态导致 webview 首次加载失败卡白；重启 dev 会话修复。
- 场景（0003 表首次启用）：scenes_repo CRUD + set_visible upsert + visibility
  JOIN（孤儿行过滤）；6 个 scene 命令；桌面页场景行（Layers 图标、全部、
  新建场景）+ 集合眼睛开关（隐藏=变暗仍可见，一键恢复）+ 再点当前场景回到
  上一个场景 + ui.activeScene 持久化（D16）。
- 真机验证：建「测试场景」→ 激活 → 集合眼睛出现 → 删除 → settings 回写 null；
  30 单测全绿，clippy/svelte-check/eslint 0。

## 2026-09-05 — Round 7: M3 桌面布局快照/恢复（LVM 通道）

- 迁移 0007：`layout_snapshots.name`（唯一索引）；0002 建的表首次接上产品功能。
- `desktop::shell_layout`：移植探针验证过的 LVM 实现（Progman/WorkerW 链、
  explorer.exe 远程缓冲、中文标题往返）；`read_layout` / `canary_check` /
  `apply_layout`（按标题匹配、重复名按序消费、缺失计数、逐项回读校验）。
- 恢复前金丝雀探测：+150/+150 试写，读回无变化即判定自动排列并整体拒绝；
  试写后总是先复位。
- 命令 layout_capture/list/apply/delete + 设置页「桌面布局快照」区（保存/应用/删除）。
- 验证：cargo test 27 单测 + `-- --ignored` 真机 2/2（实读桌面 + 金丝雀往返）、
  clippy 0、svelte-check 0、eslint 0。UI 实机走查与用户恢复实测待办。

## 2026-09-05 — Round 6: external collection items + wallpaper + data purge

User requests: (1) drag shortcuts from *outside* the desktop into collections,
(2) custom wallpaper with opacity, (3) an app-data cleanup entry.

**Shipped:**

- Migration 0006: snapshot columns on `collection_items` (label/kind/ext/
  size/modified) for paths that are not desktop-indexed. `assign_external`
  snapshots fs metadata; `items()` LEFT JOINs `desktop_items` so indexed
  paths keep live metadata and missing items stay hidden.
- `collection_assign_external` (routes indexed→live, else snapshot),
  `collection_open` (allow-list = visible index ∪ collection-held, D14),
  `desktop_icon` widened to collection-held paths.
- dragDropEnabled back to **true** (external drops need it) + hand-rolled
  pointer drag for card→chip (6 px threshold, elementFromPoint hit-test,
  floating ghost) — see D13. External drops: Tauri drag-drop events →
  paths → active collection; overlay hint while hovering.
- Wallpaper: file picker (HTML input, chunked base64 over IPC) →
  `background_set` stores `background.img` in app data (mime sniffed, 15 MB
  cap) → served via the `bg` custom protocol (`http://bg.localhost/...`) →
  fixed layer in App.svelte with a persisted opacity slider (0–100%).
- Settings 数据管理: 清空集合 / 重置全部数据 with two-step arm-confirm;
  both back the DB up (WAL checkpoint + file copy) before deleting
  (`purge_collections` / `purge_all`, unit-tested).

**Verified live:** schema v6 migration, pointer drag assignment (count
0 → 1, confirmed by user), wallpaper set through the real native file
dialog + opacity 35%→80% live preview, purge buttons render. External
drag-in from Explorer is wired but needs one manual user pass (OLE drags
can't be synthesized).

Tests: cargo test 24/24 (collections external snapshot + purge scopes),
clippy 0, fmt clean; svelte-check 0, eslint 0, vitest 3/3, vite build ok.

## 2026-09-05 — Round 5: M2 virtual collections + drag assignment (M2 complete)

**Shipped:**

- `collections_repo` (list with item counts / create / rename / delete /
  assign / unassign / items-of-collection) + 7 `collection_*` commands.
  Assignment is metadata-only and allow-listed to indexed paths (same policy
  as open/icon); delete removes assignments explicitly so it holds regardless
  of the foreign_keys pragma.
- DesktopPage: collections bar — 全部 chip, per-collection chips (color dot +
  live count), inline 新建集合 input (Enter/blur submit, Esc cancel, palette
  color rotation), per-collection 删除 button, 移出集合 drop chip while a
  collection is active. HTML5 dragstart on item cards, drop targets on chips;
  toast feedback; collection view filters the grid (search box narrows it
  client-side).

**Debugged for real:** drag assignment was dead in the running app. Synthetic
drags were not the cause — the user confirmed real drags no-oped too. Root
cause: Tauri 2's `dragDropEnabled: true` default puts a native OLE drop
handler on WebView2 that swallows in-page HTML5 dragstart/drop on Windows.
`dragDropEnabled: false` in tauri.conf.json fixed it (D12); **the user
manually verified drag-drop works**.

Tests: cargo test 21/21 (4 new repo tests: roundtrip+order, allow-list
rejection, duplicate names + rename, delete cascade + missing hidden),
clippy 0, fmt clean; svelte-check 0, eslint 0, vitest 3/3, vite build ok.

**Next:** M3 shell integration on the verified LVM route (snapshot/restore
wired to collections; canary auto-arrange detection before batch moves).

## 2026-09-05 — Round 3: M2 core (Desktop Index) + Chinese UI

**Shipped:**

- UI language switched to Chinese per user decision (D9): all pages, tray
  menu (显示主窗口/退出), `zh-CN` dates/durations, `datetime` tests updated.
- `desktop/` module:
  - `discovery.rs` — SHGetKnownFolderPath (FOLDERID_Desktop → `D:\Desktop`
    redirect honored + FOLDERID_PublicDesktop), env fallback, same-dir dedupe.
  - `scanner.rs` — top-level scan; kind = folder / shortcut (.lnk/.url) /
    file; hidden+system entries skipped (crate FILE_ATTRIBUTE_* constants);
    display_name rule per D10.
  - `desktop_repo.rs` — `sync_scan` upserts + `missing=1` history in ONE
    transaction; `list_visible`, `search` (LIKE-escaped), `find_visible`.
  - `watcher.rs` — notify recommended watcher (non-recursive), 500 ms
    quiet-period debounce, self-reconnecting thread; rescan → emit
    `desktop:changed` only on real change (D11).
  - `open.rs` — ShellExecuteW; commands validate the path is currently
    indexed before opening (webview cannot open arbitrary paths).
- DesktopPage: real grid (lucide icons by kind, size/date, 公用 badge),
  debounced backend search, refresh button, event-driven reload, Chinese
  empty/error states.
- `@lucide/svelte` added (first icon usage).

**Verified live (WINDOWS_TESTED):** 24 real items indexed (15/4/3 user +
2 public); create/delete of `dm-probe-temp.txt` on `D:\Desktop` auto-synced
(`added=1` then `removed=1`, history row kept); folder open via UI worked;
UI search "pdf" → 8 hits; Chinese date "2026年9月5日星期六". cargo test 14/14
(7 new: discovery, scanner ×3, repo ×3), clippy 0 warn, fmt clean; vitest
3/3, svelte-check 0, eslint 0, vite build ok.

**Learned:**

- windows 0.61: `SHGetKnownFolderPath(&GUID, KNOWN_FOLDER_FLAG(0), None)
  -> Result<PWSTR>` (3 args, returns the buffer; free with CoTaskMemFree);
  constants are PascalCase `FOLDERID_Desktop`/`FOLDERID_PublicDesktop`.
- `Connection::unchecked_transaction` allows `&self` repos (query_row's
  `&mut self` on prepared statements needs the raw conn, not a `tx()`).
- notify 8 API: `recommended_watcher(closure)` + `watch(path, NonRecursive)`;
  only the signal matters — the rescan reads reality from disk.
- Element `class:` directives are invalid on Svelte 5 components — pass
  `class={cond ? "x" : ""}` to lucide icons instead.

**Next:** M2 remainder (lazy bounded icon cache; collections + drag/drop),
then M3 shell integration on the LVM route.
in: Progman > SHELLDLL_DefView (direct child) > SysListView32,
    owner = explorer.exe; LVM messages work cross-process via VirtualAllocEx /
    Read/WriteProcessMemory.

**Incident (honest record):** the first write attempt used a hand-written
message constant that was actually `LVM_SETITEMPOSITION` (0x100F), not
`LVM_SETITEMPOSITION32` (0x1071). The remote pointer was interpreted as packed
coordinates; Explorer's off-view icon rescue scrambled all 27 desktop icons
into left-edge slots. Root-caused via the windows crate's authoritative
constants, fixed, and the user's layout was fully restored from the probe
snapshot (27/27 exact). Lesson codified as DECISIONS D7 (never hand-write Win32
constants); backend strategy codified as D8 (COM first-try, LVM fallback).

**Machine notes:** desktop listview client coords are the canonical position
space (MapWindowPoints no-ops cross-process from a DPI-unaware console);
virtual-screen origin (-2560,-559) must be stored alongside positions.

**Next:** commit + push (first push to the remote), then M1 — tray, global
shortcut, custom titlebar.

## 2026-09-05 — Round 3: first push + M1 Application Shell

**Shipped:**

- First push to `github.com:Okazaki-Yumemi/desktopManager` (`7fa6135`, 86
  files): full M0 foundation + verified probe + docs.
- **M1 Application Shell** (WINDOWS_TESTED on the real desktop):
  - Tray icon + menu (Show DesktopManager / Quit); left-click toggles the
    window; the window close button hides to tray instead of quitting
    (resident tool behavior, logged).
  - Global shortcut `Alt+Shift+D` (command-palette binding, toggles the
    window until the palette exists in M6). Registration is conflict-tolerant:
    a taken binding logs a warning and shows in Settings; startup never fails.
  - Theme system completion: 5 accent presets (ocean default) applied via
    `data-accent` attribute + tokens.css cascade, persisted in settings.
  - Settings: accent swatches, global-shortcut card with live registration
    status (green "registered" badge verified live).
  - Today: quick-start card (shortcut hint + close-to-tray explanation).
  - New command `shortcuts_get`; `AppState.shortcut_status`.

**Learned:**

- tao's `Window::is_focused()` tracks internal focus state and missed the
  focused case with WebView2's child-window focus — the toggle always took
  the "show" branch. Fixed by comparing `GetForegroundWindow()` against the
  window HWND (`windows` crate dep added for src-tauri, Windows-only).
- tauri-plugin-global-shortcut 2.3: plugin is `Builder::new().build()`, the
  manager extension trait is `GlobalShortcutExt`, per-shortcut registration
  via `app.global_shortcut().on_shortcut(...)` in setup.
- eslint flagged 2 latent issues in M0 toast code (`let`→`const` for runes
  state, unused type import) — fixed; the rest of the suite stays green.

**Test evidence:** log shows `global shortcut registered binding="alt+shift+d"`,
toggle debug lines (`visible=true foreground=false` → show, then
`foreground=true` → hide), `main window hidden to tray (close requested)`;
settings table contains `ui.theme="system"`, `ui.accent="ocean"` after live
switching. cargo test 6/6, clippy clean, fmt clean; vitest 3/3, svelte-check 0,
eslint 0, vite build ok.

**Next:** M2 — Desktop Index (user+public desktop discovery with redirect
awareness, debounced watcher, `desktop_items` indexing, open via shell,
search).

## 2026-09-05 — Round 4: M2 icon cache (lazy, bounded)

**Shipped:**

- `desktop/icons.rs`: on-demand shell icon extraction — SHGetFileInfoW
  (SHGFI_ICON|SHGFI_LARGEICON) → GetIconInfo → GetDIBits 32bpp top-down
  (negative biHeight), BGRA→RGBA swap, AND-mask alpha fixup for legacy
  icons; backend LRU cache (256 entries, FIFO-evicting with recency
  refresh); payload = dimensions + base64 RGBA over IPC. `desktop_icon`
  command, allow-listed to indexed paths like `desktop_open`.
- Frontend: `iconCache.ts` (bounded Map 512, recency-refreshing,
  canvas→PNG data-URL encoding — no image crate in the backend) and
  `DesktopIcon.svelte` (snippet-based generic-glyph fallback).
- deps: base64 0.22, windows feature Win32_Graphics_Gdi.

**Verified live (WINDOWS_TESTED):** real desktop grid shows authentic
shell icons — red PDF badges, Word/PPT icons, yellow folders, Zotero /
ZCode / Battlestate / 此电脑 shortcut icons resolved to their targets.
cargo test 17/17 (new: LRU eviction, real notepad.exe extraction,
missing-path→None), clippy 0, fmt clean; svelte-check 0, eslint 0,
vite build ok, vitest 3/3.

**Learned:**

- HICON pixel reading needs a masked DC dance: GetIconInfo gives
  hbmColor+hbmMask; read the color bitmap with a negative-height
  BITMAPINFOHEADER (top-down); legacy icons carry alpha=0 everywhere and
  rely on the 1bpp AND mask (bit 0 = opaque) — fix up per pixel.
- windows 0.61: DeleteObject/GetObjectW take HGDIOBJ — wrap HBITMAP as
  HGDIOBJ(h.0); SHGetFileInfoW returns BOOL-as-usize (0 = fail);
  BITMAPINFOHEADER.biBitCount is u16.
- base64 string length ≠ byte length (the first test assertion compared
  5464 chars against 4096 bytes — caught immediately by the test).
- Dev loop note: tauri dev auto-rebuilds + restarts the app on Rust
  changes and Vite HMRs the webview; the ZCode crash killed the whole
  dev process tree (job objects), a plain relaunch is enough.

**Next:** M2 remainder — virtual collections + drag/drop assignment —
then M3 shell integration on the LVM route.
