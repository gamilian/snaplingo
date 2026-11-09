# Windows 便携版

适用于 Windows 10 / 11 x64。下载 Release 中的 `snaplingo-vX.Y.Z-windows-x86_64-portable.zip`，解压整个文件夹，再运行其中的 `snaplingo.exe`，无需运行安装器。

保留程序旁边的 DLL、`ocr` 和 `licenses` 文件夹；只复制 EXE 会导致本地 OCR 或程序启动失败。压缩包已包含 OCR 模型、ONNX Runtime、DirectML 和 Visual C++ 运行库。

原生便携包不需要 WebView2。ZIP 已包含 OCR 模型、ONNX Runtime、DirectML 和 Visual C++ 运行库；请完整解压并保留这些目录与 DLL，不要只复制 EXE。

设置、数据库和历史记录仍保存在当前 Windows 用户的应用数据目录，与安装版共用。退出程序后可删除解压目录；这不会删除已有用户数据。更新时退出旧版，解压新版到另一个目录后启动。

便携版与安装版提供相同的截图、长截图、OCR、翻译和贴图功能。第三方组件的许可声明随包保留。
