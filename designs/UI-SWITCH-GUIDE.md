# SnapLingo UI 设计方案切换指南

本文档说明如何在代码中快速切换不同的UI设计方案。

## 📁 文件结构

```
designs/snaplingo-ui/
├── app-optimized.html          ⭐ 正式优化版（推荐）
├── optimized-version.html      🎨 优化版演示（含多视图切换）
├── professional-demo.html      ⚙️ 专业工具·蓝色深色
├── modern-demo.html            ✨ 现代简约·中性浅色
├── all-variants.html           📋 12种风格对比
└── final-designs.html          📖 设计总览页
```

## ⭐ 推荐方案：app-optimized.html

**访问地址**: http://localhost:4311/snaplingo-ui/app-optimized.html

### 特点：
- ✅ 基于你现有代码结构优化
- ✅ 保留三栏布局（主导航88px + 二级导航200px + 内容区）
- ✅ 自定义品牌色系统（#5b7fff 蓝紫色）
- ✅ 统一设计规范（圆角/阴影/间距）
- ✅ Provider品牌色支持
- ✅ 完整微交互动画
- ✅ 生产环境就绪

---

## 🎯 如何应用到 Tauri 项目

### 方案 1: 直接迁移 CSS 变量（推荐）

#### 步骤1: 复制 CSS 变量到全局样式

在 `src/styles/globals.css` 或 `src/index.css` 中添加：

```css
:root {
  /* 品牌色 */
  --primary-50: #f0f4ff;
  --primary-100: #e0e7ff;
  --primary-500: #5b7fff;
  --primary-600: #4a6fe8;
  --primary-700: #3a5fd1;

  /* 中性色 */
  --gray-50: #fafbfc;
  --gray-100: #f4f6f8;
  --gray-200: #e8ecf0;
  --gray-300: #dde2e8;
  --gray-400: #9ca3af;
  --gray-500: #6b7280;
  --gray-600: #4b5563;
  --gray-700: #374151;
  --gray-800: #1f2937;
  --gray-900: #111827;

  /* 语义色 */
  --success: #10b981;
  --warning: #f59e0b;
  --danger: #ef4444;

  /* Provider品牌色 */
  --google: #4285f4;
  --deepl: #0f2b46;
  --baidu: #2932e1;
  --openai: #10a37f;

  /* 间距系统 */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-6: 24px;
  --space-8: 32px;

  /* 圆角系统 */
  --radius-sm: 6px;
  --radius-md: 10px;
  --radius-lg: 14px;
  --radius-xl: 18px;
  --radius-2xl: 24px;

  /* 阴影系统 */
  --shadow-xs: 0 1px 2px rgba(0, 0, 0, 0.04);
  --shadow-sm: 0 2px 4px rgba(0, 0, 0, 0.06);
  --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.08);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.12);
  --shadow-xl: 0 12px 40px rgba(0, 0, 0, 0.16);

  /* 过渡 */
  --transition-fast: 150ms cubic-bezier(0.4, 0, 0.2, 1);
  --transition-base: 200ms cubic-bezier(0.4, 0, 0.2, 1);
}
```

#### 步骤2: 扩展 tailwind.config.js

```js
module.exports = {
  theme: {
    extend: {
      colors: {
        primary: {
          50: '#f0f4ff',
          100: '#e0e7ff',
          500: '#5b7fff',
          600: '#4a6fe8',
          700: '#3a5fd1',
        },
        google: '#4285f4',
        deepl: '#0f2b46',
        baidu: '#2932e1',
        openai: '#10a37f',
      },
      borderRadius: {
        'sm': '6px',
        'md': '10px',
        'lg': '14px',
        'xl': '18px',
        '2xl': '24px',
      },
      boxShadow: {
        'xs': '0 1px 2px rgba(0, 0, 0, 0.04)',
        'sm': '0 2px 4px rgba(0, 0, 0, 0.06)',
        'md': '0 4px 12px rgba(0, 0, 0, 0.08)',
        'lg': '0 8px 24px rgba(0, 0, 0, 0.12)',
        'xl': '0 12px 40px rgba(0, 0, 0, 0.16)',
      },
      spacing: {
        '1': '4px',
        '2': '8px',
        '3': '12px',
        '4': '16px',
        '6': '24px',
        '8': '32px',
        '12': '48px',
      },
    },
  },
}
```

#### 步骤3: 替换组件中的颜色类名

```tsx
// ❌ 原版（使用默认颜色）
<button className="bg-blue-500 text-white">

// ✅ 优化版（使用品牌色）
<button className="bg-primary-600 text-white">

// ❌ 原版
<div className="bg-gray-50">

// ✅ 优化版（使用CSS变量）
<div style={{ background: 'var(--gray-50)' }}>
```

---

### 方案 2: 创建主题切换系统

#### 创建 `src/theme/index.ts`

```typescript
export const themes = {
  optimized: {
    name: '优化版',
    colors: {
      primary: {
        50: '#f0f4ff',
        500: '#5b7fff',
        600: '#4a6fe8',
      },
      // ... 其他颜色
    },
  },
  original: {
    name: '原版',
    colors: {
      primary: {
        50: '#eff6ff',
        500: '#3b82f6',
        600: '#2563eb',
      },
      // ... 其他颜色
    },
  },
};

export type ThemeName = keyof typeof themes;
```

#### 创建主题 Context

```typescript
// src/contexts/ThemeContext.tsx
import React, { createContext, useState, useContext } from 'react';
import { themes, ThemeName } from '../theme';

const ThemeContext = createContext<{
  theme: ThemeName;
  setTheme: (theme: ThemeName) => void;
}>({
  theme: 'optimized',
  setTheme: () => {},
});

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, setTheme] = useState<ThemeName>('optimized');

  // 应用CSS变量
  React.useEffect(() => {
    const root = document.documentElement;
    const colors = themes[theme].colors;

    Object.entries(colors.primary).forEach(([key, value]) => {
      root.style.setProperty(`--primary-${key}`, value);
    });
  }, [theme]);

  return (
    <ThemeContext.Provider value={{ theme, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export const useTheme = () => useContext(ThemeContext);
```

#### 在组件中使用

```tsx
import { useTheme } from '../contexts/ThemeContext';

function SettingsPage() {
  const { theme, setTheme } = useTheme();

  return (
    <div>
      <select value={theme} onChange={(e) => setTheme(e.target.value)}>
        <option value="optimized">优化版</option>
        <option value="original">原版</option>
      </select>
    </div>
  );
}
```

---

## 🎨 各方案对比

| 特性 | 原版 | 优化版 | 专业工具 | 现代简约 |
|------|------|--------|----------|----------|
| **品牌色** | 默认蓝 | 自定义蓝紫 | GitHub深蓝 | 中性灰 |
| **圆角** | 混用 | 统一6-24px | 小圆角4-8px | 大圆角8-16px |
| **动画** | ❌ 无 | ✅ 完整 | ✅ 完整 | ✅ 完整 |
| **Provider色** | ❌ 无 | ✅ 有 | ✅ 有 | ✅ 有 |
| **适用场景** | 原型 | 生产环境 | 专业用户 | 日常使用 |

---

## 📝 快速切换命令

```bash
# 查看所有设计方案
open http://localhost:4311/snaplingo-ui/final-designs.html

# 查看优化版
open http://localhost:4311/snaplingo-ui/app-optimized.html

# 查看12种风格对比
open http://localhost:4311/snaplingo-ui/all-variants.html
```

---

## 💡 建议的迁移路径

1. **第一步**: 复制 CSS 变量到项目（10分钟）
2. **第二步**: 扩展 tailwind.config.js（5分钟）
3. **第三步**: 逐个组件替换颜色类名（1-2小时）
4. **第四步**: 验证所有页面显示正常（30分钟）
5. **第五步**（可选）: 实现主题切换系统（1小时）

总计时间：约2-4小时完成完整迁移

---

## 🔗 相关文件

- **优化说明**: designs/snaplingo-ui/final-designs.html#optimization-details
- **设计源码**: designs/snaplingo-ui/app-optimized.html
- **Tauri项目**: src/components/

---

**最后更新**: 2026-06-15
