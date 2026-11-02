# AirSnip 当前实际功能清单

调查日期：2026-11-02（Asia/Shanghai）。本轮重新从当前默认分支的入口、消息处理、设置模型与输出调用链盘点，不以旧差距表作为功能全集。仅阅读官方仓库源码、GitHub 元数据与正式发布说明，未运行 Windows 程序。该文件只记录 AirSnip；SnapLingo 是否覆盖需另核对其实现和运行结果。

## 准确基线

| 基线 | 标识 | 日期（北京时间） | 来源 |
| --- | --- | --- | --- |
| 当前默认分支 | `main`，commit `f3e0718f486845f42ba0e556a9c95b68a733facb` | 2026-11-01 17:37:48 | [官方 commit][main-commit] |
| 最新正式 Release | `v0.6.0`，`prerelease=false`、`draft=false` | 2026-10-31 19:36:59 发布 | [正式发布][release]、[API][release-api] |
| Release 对应源码 | commit `13dfee6a8a0249187a0f215c8d06d462032c176e` | 2026-10-31 19:36:01 | [固定 commit][release-commit] |
| annotated tag 对象 | `8c49d7300932e0459408e17e2294f793e4fcb894` | tag 指向上一行 commit | [tag ref API][tag-api] |
| 下载资产 | `airsnip-v0.6.0-win-x64.zip`，634864 bytes | Windows x64 绿色包 | [正式发布][release] |

`main` 比正式版新增界面缩放（PR #55），Cargo 版本仍是 `0.6.0`；不能把 AGENTS 中标为“v0.7.0”的开发事项当作已有正式 Release。下方未特别标记的实际功能在正式版和 `main` 中均具备；“仅 main”的界面缩放单独列出。核对依据包括 [两个固定 commit 的 diff][release-main-diff]。

## 1. 启动、截图与选区

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| C01 | 单文件 Windows 应用，启动后无主界面、托盘常驻；正式版要求管理员权限。Windows 10/11 与 PerMonitorV2 在 manifest 声明；正式资产仅 x64 | [启动][app]、[manifest][manifest]、[Release][release] |
| C02 | 全局截图默认 F1，可修改主键和 Ctrl/Alt/Shift/Win；从托盘常驻进程启动截图线程 | [全局热键][hotkeys]、[截图入口][start-capture] |
| C03 | 一次抓取整个虚拟桌面并冻结，支持负 x/y 的多屏布局；普通抓屏包含分层窗口。全屏像素作为任意框选底图，不等于提供一键全屏/单屏/窗口模式 | [抓屏][capture]、[覆盖层初始状态][overlay-init] |
| C04 | 拖动建立矩形选区，选区外变暗，显示宽×高；八向边/角手柄调整选区 | [选区交互][selection-input]、[选区绘制][selection-draw] |
| C05 | 按住 Space 在选区内拖动移动整个选区；单独移动选区不会自动移动已有标注 | [输入][selection-input]、[微调][selection-nudge] |
| C06 | 方向键 1px、Shift+方向键 5px：优先移动选中标注，无选中则移动选区；拖拽/绘制中忽略该微调 | [步进][hotkey-step]、[优先级][selection-nudge] |
| C07 | 远处点击重新框选；小于 2px 的误触选区丢弃 | [重新框选/误触][selection-input] |
| C08 | 选区空白双击、Enter、Ctrl+C 或工具栏复制 → 烘焙当前标注并复制、结束截图；无选区时不会自动复制全屏 | [双击/键盘][overlay-input]、[复制输出][capture-copy] |
| C09 | 右键取消；Esc 在字体面板打开时先收面板，否则取消截图；编辑文本时 Esc 先处理文本输入 | [覆盖层输入][overlay-input]、[文本键盘][text-keys] |
| C10 | 工具栏随选区布局，优先选区下方，空间不足时放上方，并避让尺寸文字；不支持任意拖动工具栏 | [工具栏布局][toolbar-layout] |

## 2. 标注对象、样式与二次编辑

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| E01 | 六种实际工具：矩形、箭头、文本、区域荧光笔、高斯模糊、聚光灯 | [对象/工具枚举][editor-kinds]、[绘制草稿][draft] |
| E02 | 矩形有描边、填充、颜色、线宽；Shift 新建约束为正方形。填充与描边不等于单独可调透明度 | [矩形模型][editor-kinds]、[Shift 约束][draft]、[样式入口][style-action] |
| E03 | 箭头先绘直线，可拖两个三次贝塞尔控制点形成曲线/S 形；双击曲率控制柄恢复直线 | [箭头模型][editor-kinds]、[控制柄输入][curve-input] |
| E04 | 箭头起点和终点可独立拖动来拉长/改向；Shift 新建/拖端点吸附水平、垂直、45° | [端点编辑][object-drag]、[新建约束][draft] |
| E05 | 区域荧光笔为半透明 Multiply 混色，透出底图文字；它是矩形区域，不是自由画笔。效果 alpha 固定 150/255 | [混色实现][region-effects] |
| E06 | 区域模糊用迭代 Box Blur 近似高斯；强度可改，导出烘焙进图片 | [区域模糊][region-effects]、[滚轮][wheel] |
| E07 | 聚光灯先暗化截图选区整体，再恢复指定区域原亮度；支持多个区域，导出与显示效果对应 | [聚光烘焙][bake]、[对象模型][editor-kinds] |
| E08 | 矩形、荧光笔、模糊、聚光灯均有独立圆角；选中后拖内部圆角柄，范围 0 到短边一半；新建默认直角 | [圆角规则][corner]、[拖柄][object-drag]、[新建][draft] |
| E09 | 单击选中、拖对象移动；选中显示包围框与手柄，区域对象和文字可八向缩放；Shift 锁轴移动/角手柄等比 | [命中/输入][selection-input]、[对象拖动][object-drag] |
| E10 | 工具栏选颜色/填充实时改选中对象，也更新后续新对象默认；模糊/聚光不使用颜色。可选颜色只有预置五种，无自定义颜色/透明度输入 | [样式动作][style-action]、[样式更新][object-style]、[调色板][defaults] |
| E11 | 滚轮悬停对象即可改其线宽/模糊强度/字号，无需先选中；空白处改当前工具默认；Ctrl 为 3 倍步进，普通步进 1 | [完整滚轮处理][wheel] |
| E12 | 矩形/箭头线宽 1–32px（默认 4）；模糊强度 4–64（默认 16）；文字滚轮/按钮 10–96px（默认 24） | [滚轮范围][wheel]、[默认值][defaults] |
| E13 | Ctrl+Z 撤销，Ctrl+Y/Ctrl+Shift+Z 重做；最多 100 步；一次拖拽仅计一步，新编辑清空 redo | [输入][overlay-input]、[历史][history]、[拖拽提交][object-drag] |
| E14 | 上次工具在同一进程内记忆；颜色、字体、字号、线宽、模糊、填充每次截图重新初始化，没有这些样式的跨截图持久化设置 | [last_tool][settings-tool]、[重新初始化][overlay-init] |

## 3. 文本与字体

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| T01 | 点击放置文字，支持普通字符/UTF-16 与中文 IME 候选和确认；非文本交互屏蔽 IME，进入文本时恢复 | [文本创建][text-input]、[字符/IME][overlay-input]、[IME 控制][ime] |
| T02 | 系统字体族列表可滚动选择；字体名排序去重，优先中文显示名；不支持字体搜索/上传 | [字体枚举][fonts]、[字体列表交互][font-panel] |
| T03 | 字体、粗体、斜体、字号可在输入中、选中已有文字时和下一个文字默认值上修改；第二行字体工具栏仅在适用状态显示 | [字体状态/应用][font-actions] |
| T04 | 字号 −/+ 每次 2px，显示数值不直接输入；滚轮普通 1px/Ctrl 3px | [字号按钮][font-actions]、[滚轮][wheel] |
| T05 | 已有文本可双击编辑；Enter 或点击别处提交；新建空文本丢弃，已有文本清空并提交会删除该对象 | [文本提交][text-input]、[鼠标提交][selection-input] |
| T06 | Shift+Enter/Ctrl+Enter 换行；左右箭头、Home/End、Backspace/Delete 操作文本；Esc 或 Ctrl+Z 放弃当前未提交编辑。没有文本 Ctrl+V 粘贴处理、文本选择/剪切快捷键 | [完整文本键盘分支][text-keys] |
| T07 | 拖手柄缩放文字会调整字号，单独缩放范围为 8–160px；仅按纵向比例改字号，文字不横向拉伸，随后重新测量；这是不同于字号按钮的独立路径 | [文字变换][text-transform]、[重测量][object-drag] |

## 4. 普通截图输出

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| O01 | 剪贴板同时提供 CF_DIB 和注册 PNG；实际预先同步编码并立即提供完整 PNG 数据，没有真正延迟渲染消息路径 | [剪贴板完整实现][clipboard-output] |
| O02 | Ctrl+S/工具栏另存为使用系统对话框，选路径、文件名、PNG/JPEG/WebP；取消对话框保留截图，可继续标注 | [保存流程][capture-save]、[原生对话框][file-dialog] |
| O03 | 另存为记录最后成功选择的目录与扩展名，下次默认定位/选中该格式；目录已不存在则回默认目录/系统位置 | [偏好读取/记录][capture-save]、[写回配置][save-prefs] |
| O04 | 输出编码依据文件扩展名，支持 `.png`、`.jpg`、`.jpeg`、`.webp`；未知/无扩展名报错；原生对话框请求覆盖确认 | [编码分发][image-output]、[覆盖提示][file-dialog] |
| O05 | JPEG 固定质量 90，WebP 为无损 VP8L，PNG 快速压缩；没有质量滑块或有损 WebP | [格式编码][image-output] |
| O06 | 普通另存为可预填三种命名模板：日期+时间、Unix 毫秒时间戳、仅日期 | [命名实现][image-output]、[设置入口][settings-controls] |
| O07 | 普通复制只复制；普通另存为只保存，没有保存后自动复制设置；输出在后台线程，失败会弹窗，截图覆盖层已退出 | [复制/保存流程][capture-copy]、[保存流程][capture-save] |
| O08 | 截图 F3/贴图按钮把选区与标注烘焙后送贴图窗口，结束覆盖层；截图内 F3 硬编码，与全局贴图快捷键改键不是同一配置路径 | [截图贴图][capture-pin]、[F3 输入][overlay-input] |

## 5. 手动滚动长截图

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| L01 | 截图选区内按可配置长截图键（默认 S）/工具栏按钮进入；仅保留视口坐标，普通截图标注不带入长图 | [切模式][capture-longshot]、[调用长截图][start-capture] |
| L02 | 用户滚动页面，90ms 定时抓取固定视口；无自动滚动，也不自动检测到底结束 | [采帧/输入][longshot-window]、[静帧处理][longshot-session] |
| L03 | 下滚追加新条带，上滚前置新条带；完整重叠区一致性检验定位，不只匹配局部模板 | [匹配][stitch]、[新增条带][longshot-engine] |
| L04 | 检测固定页眉/页脚并只保留一次，拼接时排除固定区域；初始化静帧不整张当固定区 | [固定区处理][longshot-engine]、[固定区检测][stitch] |
| L05 | 无位移帧不新增、不自动完成；失败继续采集并提示慢滚动；实际抓屏错误取消整个任务 | [状态处理][longshot-session] |
| L06 | Ctrl+Z/撤销按钮撤掉最后拼入段；没有长截图 redo。Undo 只改拼接内容，不自动倒滚源页面 | [撤销][longshot-engine]、[控件][longshot-window] |
| L07 | Enter/完成按钮收尾前补采最后一帧；Esc/取消按钮放弃。执行期间临时注册 Enter/Esc/Ctrl+Z 全局热键，不是仅靠隐藏 WebView 键盘 | [执行键与收尾][longshot-window] |
| L08 | 实时等比缩略预览，显示在左右较宽一侧（空间并列选右），底边对齐视口；不足最小宽度时隐藏，任务继续 | [预览布局][longshot-preview] |
| L09 | 预览为透明背景并点击/滚轮穿透；只在高度改变时更新，按目标像素抽样，不先合成整张长图 | [预览窗口][longshot-preview]、[thumbnail][longshot-engine] |
| L10 | 有视口边框和状态条；状态条含撤销、完成、取消，显示操作/失败提示，当前版本不显示像素计数。状态条优先视口下方、否则上方 | [状态条][longshot-bar]、[创建窗口][longshot-window] |
| L11 | `longshot_max_height` 默认 30000，可手改 TOML，没有面板输入；达到内容高度上限自动结束并输出已有内容 | [配置][settings-config]、[Limit][longshot-session]、[上限校验][longshot-engine] |
| L12 | 正常完成始终“复制 + 保存默认目录的日期时间 PNG”，无需再打开另存为；不使用上次目录/格式/命名模板；复制失败仍会尝试保存 | [完整完成输出][longshot-output]、[默认目录/命名][image-output] |
| L13 | 完成后不会进入标注编辑器，不直接贴图；没有横向滚动、暂停按钮、自动视口识别。取消不会保存 | [真实调用与输出][start-capture]、[执行输入][longshot-window]、[完成输出][longshot-output] |

## 6. 贴图

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| P01 | 来源为截图选区或全局贴图热键读取的剪贴板图片；PNG→CF_DIB→CF_BITMAP 回退，透明 PNG 支持；没有文字/颜色贴图和直接文件导入入口 | [全局入口][hotkeys]、[读图][pin-clipboard]、[截图来源][capture-pin] |
| P02 | 截图来源默认原位置，可设置虚拟屏幕中央；剪贴板始终居中。多个居中图有 20px 级联偏移，初始位置夹回虚拟屏幕 | [定位][pins]、[设置入口][settings-controls] |
| P03 | 无系统边框、始终置顶、不出现在任务栏；默认不透明，内容 PNG 的自身 alpha 保留 | [窗口创建][pin-window] |
| P04 | 左键拖动位置；方向键 1px、Shift 5px移动 | [贴图输入][pin-input]、[步进][hotkey-step] |
| P05 | 滚轮围绕中心缩放，比例限制 0.1–8，边长限制 4096px；滚动中快速预览，静止 120ms 后高质量重采样 | [缩放][pin-zoom]、[输入][pin-input] |
| P06 | Ctrl+滚轮调整窗口透明度，每步 24/255，范围 12–255；无独立设置面板默认透明度滑块 | [贴图输入][pin-input]、[alpha][pin-window] |
| P07 | 1px 蓝框默认开启，可配置快捷键（默认 Ctrl+F3）从“有焦点的贴图窗口”触发；作用于全部贴图，不是单张，也不是全局热键注册 | [局部键][pins]、[按键处理][pin-input]、[全体切换][pin-all] |
| P08 | 边框切换保持内容左上角/当前缩放；保存磁盘的 `pin_border` 默认。但 current app 只写配置副本，不更新 `self.config`，同一进程后续新图跟随该默认不能视为可靠已实现 | [边框重建][pin-window]、[默认写回][pin-border-config] |
| P09 | 双击、Esc、右键关闭单张；右键“关闭全部”遍历全部存活贴图，与组无关 | [输入][pin-input]、[右键][pin-menu]、[关闭全部][pin-all] |
| P10 | 右键复制/保存的是当前缩放后内容，去蓝框且不烘焙窗口透明度；不会恢复成原始图尺寸输出 | [导出][pin-window]、[菜单输出][pin-menu] |
| P11 | 右键保存始终默认 `Pictures/airsnip` 日期时间 PNG，自动保存而非另存为；不跟普通截图上次目录/格式/命名模板 | [菜单输出][pin-menu]、[目录/命名][image-output] |
| P12 | 最多 20 张，达到上限弹提示；没有贴图分组、显示隐藏全体、缩略图、灰度/反色、点击穿透开关或拖边框缩放 | [数量上限][pins]、[完整贴图输入][pin-input] |

## 7. 快捷键、托盘与设置

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| S01 | 托盘左键打开设置，右键实际仅设置/退出；代码虽有截图命令分支，当前菜单没有连到它，更没有长截图项 | [真实菜单][tray] |
| S02 | 设置分类：通用、快捷键、保存、外观、关于；包含自启、只读简体中文、贴图位置、10 个可改键入口、命名模板、主题、项目信息 | [实际设置行][settings-controls] |
| S03 | 可改：截图/贴图全局键及修饰键、贴图窗口内边框键及修饰键、截图内长截图主键、6 个工具主键；工具/长截图不是仅字母，接受数字/F 键等 virtual key | [捕获与赋值][settings-input]、[键配置][settings-keys] |
| S04 | 工具/长截图只存主键，录制时修饰键不形成其组合绑定。Esc取消、纯修饰继续等主键；IME屏蔽避免改键录成 VK_PROCESSKEY | [录制][settings-input] |
| S05 | 冲突检测覆盖这 10 项；全局截图/贴图实际注册失败也弹窗；内置 Enter/Space/F3/Ctrl+C/Ctrl+S/方向键等优先处理，不能称“全部快捷键任意改” | [冲突模型][key-conflict]、[注册失败][hotkeys]、[优先输入][overlay-input] |
| S06 | 默认数字键实际为 1矩形、2箭头、3文字、4模糊、5荧光笔、6聚光；README 中 4/5 的顺序与源码相反 | [默认值][settings-keys]、[README 快捷键表][readme] |
| S07 | 截图期间临时注销全局贴图键，避免 F3 同时贴选区和剪贴板；结束重注册。截图内 F3本身不可配置 | [进入/退出][start-capture]、[重新注册][save-prefs]、[输入][overlay-input] |
| S08 | 开机自启通过 schtasks 创建/删除 ONLOGON、最高权限任务，设置显示任务真实状态；失败弹窗，不靠 TOML bool 猜是否生效 | [自启动][autostart]、[面板查询][settings-panel]、[开关反馈][settings-input] |
| S09 | 主题支持系统/浅/深；打开时解析系统注册表主题，手动面板切主题立刻更新当前面板。系统主题广播没有刷新打开的面板/截图颜色的完整路径 | [解析][theme]、[面板手动更新][settings-input]、[消息分支][settings-panel]、[应用广播][app-input] |
| S10 | TOML 配置在 `%APPDATA%/airsnip`，缺失/损坏回默认；热键/主题等实时写回，关闭面板兜底同步；最后保存目录/格式不被面板旧副本覆盖 | [配置][settings-config]、[面板写回][settings-panel]、[偏好保留][start-capture] |
| S11 | 关于显示版本/GPL 许可、项目地址、第三方许可页面和赞赏二维码；无自动更新、语言切换、历史库/收藏功能 | [关于页][settings-controls]、[显示][settings-about]、[入口模块][main] |
| S12 | 单实例 mutex阻止第二进程持续运行；“重复启动唤起设置”只看到发送自定义 WM_APP+2，当前 wndproc无接收处理，不作为已实现可用行为 | [mutex/发送][app]、[完整接收分支][app-input] |
| S13 | 托盘图标在 explorer 重启/睡眠恢复后重新添加，初始化失败有限次数重试 | [托盘恢复][tray-retry]、[广播处理][app-input] |

## 8. 仅 main 的未发布界面缩放

| ID | 真实用户行为与边界 | 源码证据 |
| --- | --- | --- |
| U01 | 外观可选自动/100/125/150/175/200%；设置面板当场缩放，截图工具栏、手柄、HUD、字体面板、长截图控件和光标使用该系数；标注内容不随 UI 比例改变 | [缩放配置][ui-config]、[面板应用][settings-input]、[ADR][ui-adr] |
| U02 | 自动取启动时光标所在显示器的 DPI 与分辨率补偿较大值，范围 1–3；4K@100% 约1.52；仅初始化解析，不是截图中随鼠标跨屏连续重算 | [系数算法][ui]、[初始化][overlay-init] |

## 9. 不能从 README/PRD 直接计入的功能

| 项目 | 核查结论与证据 |
| --- | --- |
| OCR、翻译、第三方 AI服务/密钥保存 | `ocr/mod.rs` 只有二期占位；没有调用入口、网络 provider实现。[占位][ocr]、[模块入口][main] |
| 录屏、GIF、AI编辑、历史库/收藏 | 没有实际模块/界面/消息入口；不是此次功能对齐要求来源。[模块入口][main]、[设置入口][settings-controls] |
| 一键全屏、单屏/窗口模式、自动窗口吸附、放大镜取色 | PRD中有候选/后续需求，但现在 selection=None 开始，只有手动框选，键盘未处理相关入口。[初始状态][overlay-init]、[输入全集][overlay-input]、[PRD][prd] |
| 椭圆、画笔、序号、马赛克、渐变、文字背景 | 枚举只有六种工具；PRD候选与渐变为二期，文字背景推迟。[工具枚举][editor-kinds]、[PRD][prd]、[字体 ADR][font-adr] |
| 任意对象删除/层级调整 | 已主动移除入口；只有撤销，文字清空提交仍会移除该文字。[移除决策][delete-adr]、[文字提交][text-input] |
| 全部快捷键任意自定义 | 真正可配置范围见S03；保存/复制/撤销/方向等是内置处理。[键捕获][settings-input]、[输入全集][overlay-input] |
| 剪贴板 PNG 延迟渲染 | README 宣称，代码实际预编码并直接提供完整 PNG。[完整实现][clipboard-output] |
| 自动滚动/到底自动完成、横向/自动识别视口、长图再编辑 | 自动滚动已删除，后几项未实现；长图实际直接复制和自动PNG保存。[执行路径][longshot-window]、[输出][longshot-output]、[PRD][prd] |
| 长图超大图临时文件流式写出 | PRD愿望，当前仍为内存 RowBuffer；高度上限实际生效，不代表实现磁盘缓冲。[行缓冲][rows]、[PRD][prd] |
| 贴图默认透明度/鼠标穿透/组管理/中键重置 | 没有当前设置或输入入口；预览窗鼠标穿透不是贴图鼠标穿透。[贴图输入][pin-input]、[贴图创建][pin-window] |
| 所有打开界面随系统主题即时切换 | 当前只在打开时解析/面板手动修改；广播没有刷新全部界面。[主题][theme]、[应用广播][app-input]、[面板消息][settings-panel] |
| 混合 DPI 已实机验证 | README明确未验证；源码包含PerMonitorV2和main新增缩放不等于实机通过。[README限制][readme-limits] |

## 10. 旧研究特别容易遗漏的核对点

此次应独立核对的行为包括：**数字/F键改工具和长截图键；贴图窗口内组合边框键作用于全体；跨全体关闭；悬停对象滚轮调模糊且Ctrl加速；8–160文字手柄缩放；三种保存路径的区别（普通另存为/长图完成/贴图右键保存）；最后目录和格式记忆；贴图导出使用当前缩放内容；长图临时全局键与补最后一帧；上限自动输出；last_tool仅进程记忆；主分支UI缩放尚未发布。** 以上各项在对应ID有固定源码证据。

核查时还应纠正四项容易高估的陈述：Ctrl+F3不是全局热键；PNG不是实际延迟渲染；“全屏抓屏”不等于一键全屏选择；“保存三格式”不等于长图/贴图也能另存三格式。本报告不能替代 SnapLingo 桌面验收，也不证明所有行为已经在 SnapLingo 覆盖。

## 固定源码来源

[main-commit]: https://github.com/bd-dxg/airsnip/commit/f3e0718f486845f42ba0e556a9c95b68a733facb
[release]: https://github.com/bd-dxg/airsnip/releases/tag/v0.6.0
[release-api]: https://api.github.com/repos/bd-dxg/airsnip/releases/latest
[release-commit]: https://github.com/bd-dxg/airsnip/commit/13dfee6a8a0249187a0f215c8d06d462032c176e
[tag-api]: https://api.github.com/repos/bd-dxg/airsnip/git/ref/tags/v0.6.0
[release-main-diff]: https://github.com/bd-dxg/airsnip/compare/13dfee6a8a0249187a0f215c8d06d462032c176e...f3e0718f486845f42ba0e556a9c95b68a733facb
[main]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/main.rs#L8-L23
[manifest]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/app.manifest#L5-L22
[app]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/mod.rs#L92-L165
[start-capture]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/mod.rs#L168-L248
[app-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/wndproc.rs#L78-L180
[capture]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/capture/mod.rs#L143-L221
[overlay-init]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/init.rs#L54-L83
[selection-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/interaction/mod.rs#L69-L259
[selection-nudge]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/interaction/mod.rs#L35-L66
[selection-draw]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/selection/draw.rs#L1-L95
[overlay-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/wndproc.rs#L68-L218
[toolbar-layout]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/toolbar/mod.rs#L178-L211
[hotkeys]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/hotkey.rs#L14-L65
[hotkey-step]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/hotkey.rs#L70-L80
[capture-copy]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/annotate.rs#L207-L222
[capture-save]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/annotate.rs#L224-L272
[capture-pin]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/annotate.rs#L140-L187
[capture-longshot]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/annotate.rs#L190-L203
[editor-kinds]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/mod.rs#L68-L153
[draft]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/interaction/draft.rs#L12-L137
[curve-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/control.rs#L10-L90
[object-drag]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/objedit/drag.rs#L11-L103
[object-style]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/objedit/drag.rs#L134-L175
[style-action]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/annotate.rs#L61-L94
[region-effects]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/render/region.rs#L13-L122
[bake]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/render/bake.rs#L11-L65
[corner]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/corner.rs#L13-L47
[wheel]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/pick.rs#L117-L192
[defaults]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/mod.rs#L234-L256
[history]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/document.rs#L10-L99
[settings-tool]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/mod.rs#L11-L22
[text-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/textinput.rs#L16-L180
[text-keys]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/textinput.rs#L182-L214
[ime]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/ime.rs#L1-L32
[fonts]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/render/fonts.rs#L14-L60
[font-panel]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/font/panel.rs#L45-L75
[font-actions]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/font/mod.rs#L15-L214
[text-transform]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/editor/transform.rs#L8-L109
[clipboard-output]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/output/mod.rs#L18-L87
[image-output]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/output/mod.rs#L91-L205
[file-dialog]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/filedlg.rs#L66-L112
[save-prefs]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/wndproc.rs#L147-L162
[longshot-window]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/window.rs#L28-L287
[longshot-session]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/session.rs#L142-L213
[longshot-engine]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/engine.rs#L78-L230
[stitch]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/stitch.rs#L71-L210
[longshot-preview]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/preview.rs#L1-L167
[longshot-bar]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/bar.rs#L33-L166
[longshot-output]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/mod.rs#L22-L47
[rows]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/longshot/rows.rs#L10-L93
[pins]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/mod.rs#L31-L115
[pin-clipboard]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/clipboard.rs#L21-L61
[pin-window]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/window.rs#L18-L226
[pin-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/wndproc.rs#L15-L111
[pin-zoom]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/zoom.rs#L13-L68
[pin-all]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/mod.rs#L154-L185
[pin-border-config]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/mod.rs#L250-L258
[pin-menu]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/pin/menu.rs#L16-L66
[tray]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/tray.rs#L100-L139
[tray-retry]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/tray.rs#L42-L85
[settings-controls]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/panel/controls.rs#L201-L253
[settings-input]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/panel/interact.rs#L60-L246
[settings-keys]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/mod.rs#L37-L98
[key-conflict]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/panel/controls/keys.rs#L60-L110
[settings-panel]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/panel/mod.rs#L76-L205
[settings-config]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/mod.rs#L209-L288
[settings-about]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/panel/draw.rs#L270-L283
[autostart]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/app/autostart.rs#L10-L36
[theme]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/theme/mod.rs#L93-L135
[ui-config]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/settings/mod.rs#L140-L192
[ui]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/overlay/ui.rs#L25-L121
[ui-adr]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/docs/adr/0016-dpi-scaling-overlay.md#L1-L82
[ocr]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/src/ocr/mod.rs#L1-L4
[prd]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/screenshot-tool-PRD.md#L55-L158
[delete-adr]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/docs/adr/0007-drop-delete-and-layering.md#L1-L12
[font-adr]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/docs/adr/0008-font-picker.md#L1-L17
[readme]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/README.md#L82-L102
[readme-limits]: https://github.com/bd-dxg/airsnip/blob/f3e0718f486845f42ba0e556a9c95b68a733facb/README.md#L133-L138
