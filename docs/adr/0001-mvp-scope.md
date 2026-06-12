# ADR 0001: MVP 范围定义

## Status
Accepted

## Context
项目功能范围较大，需要明确 MVP（最小可行产品）的边界，确保第一个版本聚焦核心价值，快速交付可用产品。

## Decision

### P0 - 必须有（第一版发布）
1. **Screenshot Mode** - 完整截图编辑功能
   - 所有 Snipaste 级别的编辑工具（矩形、椭圆、折线、箭头、画笔、马克笔、文字、马赛克、高斯模糊、橡皮擦）
   - 保存、复制功能
   
2. **OCR + Translation Mode** - 截图 OCR 并自动翻译
   - 至少一个本地 OCR Provider（Tesseract）
   - 至少一个免费翻译 Provider（Google Translate）
   
3. **Selection Translation Mode** - 划词翻译
   
4. **Result Window** - 统一结果展示窗口
   - 可编辑文本区域
   - 语言选择控件
   - 翻译卡片列表
   
5. **基础设置**
   - 快捷键配置
   - Provider 配置（API Key 等）
   - OCR/翻译 Provider 选择

### P1 - 可以包含（第一版发布）
1. **Input Translation Mode** - 弹窗输入文字翻译
2. **贴图功能** - 截图后固定到屏幕
3. **历史记录** - 记录和搜索历史操作
4. **多 Provider 支持** - 同时激活多个翻译 Provider，对比结果
5. **自定义 Translation Provider** - 支持 OpenAI/Claude/Gemini 兼容格式
6. **完整设置界面** - 所有配置项的 UI

### P2 - 延后（后续版本）
1. 主题切换（浅色/深色）
2. 网络代理设置
3. TTS 朗读功能
4. 更多内置 Provider（百度、腾讯、DeepL 等）
5. 界面多语言

## Consequences

**优点**：
- 快速验证核心价值（截图 OCR 翻译 + 划词翻译）
- P0 + P1 已经可以覆盖日常使用场景
- 技术架构在 P0 阶段就完整建立，后续只是增量添加功能

**风险**：
- P1 功能如果不做，用户体验会明显弱于 Bob（Bob 支持多 Provider 对比）
- 建议 P1 至少包含"多 Provider 支持"和"自定义 Provider"

**时间估算**：
- P0：4-5 周
- P1：2-3 周
- 总计：6-8 周可发布第一版
