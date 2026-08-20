# Tesseract OCR 安装指南

SnapLingo 使用 Tesseract OCR 作为本地 OCR 提供商。本文档介绍如何在不同平台上安装 Tesseract。

## macOS 安装

### 使用 Homebrew（推荐）

```bash
# 安装 Tesseract
brew install tesseract

# 安装语言包（可选）
brew install tesseract-lang
```

### 验证安装

```bash
tesseract --version
```

应该看到类似输出：
```
tesseract 5.x.x
```

## Windows

Windows 版使用系统 OCR，不需要安装 Tesseract。请在 Windows 设置中安装需要识别语言的 OCR 功能包。

如需在其他工具中使用 Tesseract，可按其官方安装说明安装；SnapLingo 的 Windows 发行包不依赖它。

### 在其他工具中使用 Tesseract

1. 下载安装程序：https://github.com/UB-Mannheim/tesseract/wiki
2. 运行安装程序
3. 添加 Tesseract 到 PATH 环境变量

### 验证安装

```cmd
tesseract --version
```

## Linux 安装

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install tesseract-ocr

# 安装语言包
sudo apt-get install tesseract-ocr-chi-sim  # 简体中文
sudo apt-get install tesseract-ocr-jpn      # 日文
```

### Fedora/RHEL

```bash
sudo dnf install tesseract

# 安装语言包
sudo dnf install tesseract-langpack-chi_sim
sudo dnf install tesseract-langpack-jpn
```

### 验证安装

```bash
tesseract --version
```

## 语言支持

### 默认支持的语言

Tesseract 默认安装了英语（eng）支持。

### 安装额外语言包

**macOS (Homebrew):**
```bash
brew install tesseract-lang
```

**Ubuntu/Debian:**
```bash
# 查看可用语言包
apt-cache search tesseract-ocr

# 安装特定语言
sudo apt-get install tesseract-ocr-chi-sim  # 简体中文
sudo apt-get install tesseract-ocr-chi-tra  # 繁体中文
sudo apt-get install tesseract-ocr-jpn      # 日文
sudo apt-get install tesseract-ocr-kor      # 韩文
sudo apt-get install tesseract-ocr-fra      # 法文
sudo apt-get install tesseract-ocr-deu      # 德文
sudo apt-get install tesseract-ocr-spa      # 西班牙文
```

**Windows:**
语言包包含在安装程序中，安装时选择需要的语言。

### 支持的语言代码映射

SnapLingo 自动将常见语言代码映射到 Tesseract 格式：

| 输入代码 | Tesseract 代码 | 语言 |
|---------|---------------|------|
| en | eng | 英语 |
| zh-CN | chi_sim | 简体中文 |
| zh-TW | chi_tra | 繁体中文 |
| ja | jpn | 日文 |
| ko | kor | 韩文 |
| fr | fra | 法文 |
| de | deu | 德文 |
| es | spa | 西班牙文 |
| ru | rus | 俄文 |
| ar | ara | 阿拉伯文 |

## 故障排除

### Tesseract 未找到

**错误信息：**
```
Tesseract OCR is not available. Please install Tesseract.
```

**解决方案：**
1. 确认 Tesseract 已安装：`tesseract --version`
2. 确认 Tesseract 在 PATH 中
3. macOS：`brew install tesseract`
4. 重启 SnapLingo

### 语言包未找到

**错误信息：**
```
Failed to initialize tesseract with language: chi_sim
```

**解决方案：**
1. 安装对应的语言包（见上文）
2. 检查已安装的语言：`tesseract --list-langs`
3. 确认语言包路径正确

### macOS 特定问题

如果通过 Homebrew 安装后仍然找不到 Tesseract：

```bash
# 确认安装路径
which tesseract

# 添加到 PATH（如果需要）
export PATH="/opt/homebrew/bin:$PATH"
```

## 备选方案

如果无法安装 Tesseract，SnapLingo 提供远程 OCR 选项：

1. **Baidu OCR** - 需要 API Key 和 Secret Key
   - 访问：https://ai.baidu.com/tech/ocr
   - 在 SnapLingo 设置中配置凭证

## 性能优化

### macOS Apple Silicon

在 Apple Silicon (M1/M2) Mac 上，推荐使用 ARM 版本：

```bash
# 卸载旧版本
brew uninstall tesseract

# 重新安装
brew install tesseract
```

### 内存使用

Tesseract 可能占用较多内存。如果遇到性能问题：

1. 减少同时处理的图像数量
2. 考虑使用远程 OCR 服务（Baidu OCR）
3. 优化图像质量（降低分辨率）

## 更多信息

- Tesseract 官方文档：https://tesseract-ocr.github.io/
- GitHub 仓库：https://github.com/tesseract-ocr/tesseract
- 语言训练数据：https://github.com/tesseract-ocr/tessdata
