# macOS 远程双屏截图窗口核查

用户环境：另一台 macOS 15，双屏，通过 UU 操作；同一连接下 Snipaste 可正常截图。SnapLingo 进入冻结界面后没有选区反馈，Esc 可退出。尚未取得该设备的原生窗口和显示器日志，不能据此认定 UU 创建了虚拟屏或物理屏处于休眠。

## 本机原生验证

在 macOS 14.8.3 使用临时 Tauri example，通过生产 `open_capture_window_for_session` 和 `prepare_capture_window_for_reveal` 创建隐藏窗口，传入用户提供的几何范围 `(-1080, -139, 3000, 1920)`。等待主线程队列执行后读取实际 NSWindow frame；不启动应用数据库或进行鼠标注入。

| 阶段 | 修改前的桌面坐标范围 | 修改后 |
| --- | --- | --- |
| 创建后同步设置原生 frame | (-1080, -139, 3000, 1920) | (-1080, -139, 3000, 1920) |
| 主线程队列执行后 | (0, 0, 3000, 1920) | (-1080, -139, 3000, 1920) |
| 显示前准备完成后 | (-1080, -139, 3000, 1920) | (-1080, -139, 3000, 1920) |

Tauri 2.11.2 在创建窗口时调用 `set_outer_position(initial_position)`，tao 0.35.3 将其放入主线程异步队列。原先 macOS builder 使用 (0, 0, 1, 1)，异步位置写回会覆盖生产代码同步设置的原点。macOS builder 现直接使用会话几何范围，保留显示前的原生 frame 设置。Windows 初始化和 DPI 路径不变。

原来的准备步骤在探针中能够恢复位置，因此该结果证明了创建阶段的覆盖问题，**尚不证明远端设备在最终显示时仍然错位**。临时 example 和库入口在验证后已删除。

## 远程画面可见性

生产截图窗口原先使用 `NSWindowSharingType::None`；探针确认窗口实际 sharing 值为 0。现改为 `ReadOnly`，探针确认值为 1。共享策略回归测试修改前失败，修改后通过。Apple 的 NSWindow.h 说明 `None` 阻止其他进程读取窗口内容并影响部分系统服务；不同录屏实现可能有不同表现，因此仍需 UU 实测。

截图底图在窗口显示前冻结，新的截图会话先隐藏旧遮罩，输出从冻结像素裁剪，不依赖禁止其他应用捕获选区窗口来防止遮罩进入输出。

`cargo fmt --check`、设置 `MACOSX_DEPLOYMENT_TARGET=12.0` 的 `cargo check --all-targets` 和后端测试通过（539 项单元测试、13 项集成测试）。`npm run tauri:build` 完成前端生产构建、原生编译和 DMG 校验；包内 Mach-O 最低系统版本符合 12.0，应用和挂载的安装包均通过本地稳定证书签名校验。该构建目标检查不能替代 macOS 12、macOS 15 或 Windows 10/11 的真机验证。

本次 Apple Silicon 测试包另存为 `target/remote-test/SnapLingo_0.1.4_aarch64_remote-test.dmg`，用于与此前同版本号的产物区分；它是本地签名、未公证的测试包。

## 远端验收

使用新包分别在主屏、副屏和跨屏拖动，检查选区反馈、预览、复制、保存和 Esc；维持现有 UU 连接即可。日志中的 `[capture-input] snapshot` 记录显示器 ID、原点、像素尺寸、缩放和活动/休眠状态；`frame requested` 记录会话要求的桌面坐标范围；`revealed` 记录显示时的 AppKit frame、焦点和共享状态。AppKit frame 的 Y 原点在屏幕下方，不能直接与桌面坐标 Y 比较。

本次不更改显示器枚举策略。Online 显示器不保证可绘制或可成功截图，不能把无法取得冻结像素的睡眠屏无条件加入选区。参考 [Apple 对休眠显示器的定义](https://developer.apple.com/documentation/coregraphics/cgdisplayisasleep(_:)) 和 [NSWindow 共享属性](https://developer.apple.com/documentation/appkit/nswindow/sharingtype-swift.property)。
