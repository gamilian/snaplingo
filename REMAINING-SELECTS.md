# 剩余原生 Select 替换指南

以下文件仍包含原生 `<select>` 控件，需要手动替换为 `CustomSelect`：

## 📋 待替换文件列表

### 1. SaveSettingsPage.tsx
路径: `src/components/SettingsWindow/Screenshot/SaveSettingsPage.tsx`

**替换步骤**:
```tsx
// 1. 添加导入
import { useState } from 'react';
import { CustomSelect } from '../../common/CustomSelect';

// 2. 添加状态
const [format, setFormat] = useState('png');
const [quality, setQuality] = useState('high');

// 3. 替换 select
<CustomSelect
  options={[
    { value: 'png', label: 'PNG' },
    { value: 'jpg', label: 'JPEG' },
    { value: 'webp', label: 'WebP' },
  ]}
  value={format}
  onChange={setFormat}
/>
```

---

### 2. AdvancedPage.tsx
路径: `src/components/SettingsWindow/Advanced/AdvancedPage.tsx`

**替换步骤**:
```tsx
// 添加导入和状态
import { useState } from 'react';
import { CustomSelect } from '../../common/CustomSelect';

const [logLevel, setLogLevel] = useState('info');
const [updateChannel, setUpdateChannel] = useState('stable');

// 替换所有 select
<CustomSelect
  options={[
    { value: 'debug', label: 'Debug' },
    { value: 'info', label: 'Info' },
    { value: 'warn', label: 'Warn' },
    { value: 'error', label: 'Error' },
  ]}
  value={logLevel}
  onChange={setLogLevel}
/>
```

---

### 3. GeneralPage.tsx
路径: `src/components/SettingsWindow/General/GeneralPage.tsx`

**替换步骤**:
```tsx
import { useState } from 'react';
import { CustomSelect } from '../../common/CustomSelect';

const [language, setLanguage] = useState('zh-CN');
const [theme, setTheme] = useState('system');

<CustomSelect
  options={[
    { value: 'system', label: '跟随系统' },
    { value: 'light', label: '浅色' },
    { value: 'dark', label: '深色' },
  ]}
  value={theme}
  onChange={setTheme}
/>
```

---

### 4. CustomTranslationProviderDialog.tsx
路径: `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx`

**替换步骤**:
```tsx
import { useState } from 'react';
import { CustomSelect } from '../../common/CustomSelect';

const [providerType, setProviderType] = useState('openai');

<CustomSelect
  options={[
    { value: 'openai', label: 'OpenAI Compatible' },
    { value: 'custom', label: 'Custom API' },
  ]}
  value={providerType}
  onChange={setProviderType}
/>
```

---

## 🔄 自动化替换脚本

如果你想快速处理，可以使用以下命令查找所有包含 select 的文件：

```bash
# 查找所有包含 select 的文件
find src/components -name "*.tsx" | xargs grep -l "<select"

# 统计每个文件中 select 的数量
find src/components -name "*.tsx" | xargs grep -c "<select" | grep -v ":0"
```

---

## ✅ 已完成的文件

- ✅ EditorPage.tsx
- ✅ OcrSettingsPage.tsx
- ✅ TranslationSettingsPage.tsx
- ✅ ResultWindow.tsx

---

## 📝 替换模式

### 标准替换模式

```tsx
// ❌ 旧版（原生 select）
<select
  value={value}
  onChange={(e) => setValue(e.target.value)}
  className="w-full px-4 py-2 border border-gray-300 rounded-lg..."
>
  <option value="opt1">选项1</option>
  <option value="opt2">选项2</option>
</select>

// ✅ 新版（CustomSelect）
<CustomSelect
  options={[
    { value: 'opt1', label: '选项1' },
    { value: 'opt2', label: '选项2' },
  ]}
  value={value}
  onChange={setValue}
/>
```

---

## 🎯 关键改进

1. **移除 className** - CustomSelect 内部已有完整样式
2. **简化 onChange** - 直接传 `setValue`，不需要 `(e) => setValue(e.target.value)`
3. **选项格式** - 从 `<option>` 改为对象数组
4. **自动动画** - 自带淡入和滑下动画
5. **统一设计** - 自动使用品牌色和圆角

---

## 🚀 完成后测试

```bash
npm run tauri dev
```

检查所有下拉框：
- ✅ 点击展开流畅
- ✅ 活跃项高亮正确
- ✅ ESC键关闭正常
- ✅ 点击外部关闭正常
- ✅ 样式与整体统一

---

**更新时间**: 2026-06-15
**待处理数量**: 4个文件
