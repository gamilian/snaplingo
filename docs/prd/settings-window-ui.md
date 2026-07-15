# PRD: Settings Window UI Implementation

## Problem Statement

SnapLingo 目前缺少一个主设置窗口（Settings Window），用户无法配置应用的核心功能：截图、OCR、翻译、Provider 管理、快捷键设置等。当前的代码仅包含基础的翻译状态管理（Zustand store）和 Result Window 组件，但没有统一的配置界面。

用户需要能够：
- 配置各个功能的快捷键（截图/OCR/翻译）
- 管理 OCR 和翻译 Provider（激活、配置 API Key、优先级）
- 设置截图保存路径和编辑器默认值
- 查看和管理历史记录与收藏夹
- 配置通用设置（语言、主题、开机自启）

## Solution

实现一个功能域独立的主设置窗口（Settings Window），采用 6 个主标签页结构：截图、翻译、OCR、服务、通用、高级。每个功能域拥有完整的二级导航，包含该域的快捷键、设置、历史、收藏。

设计已通过交互式 UI 原型验证（`designs/prototypes/App.PROTOTYPE_UI.tsx`），确认布局合理、交互流畅、图标识别度高。

## User Stories

1. As a user, I want to see a main settings window when the app launches, so that I can configure the application before first use
2. As a user, I want to click on the "截图" tab in the left navigation, so that I can access all screenshot-related settings
3. As a user, I want to see secondary navigation within the screenshot tab (快捷键/保存设置/编辑器/收藏夹), so that I can find specific settings easily
4. As a user, I want to click on a hotkey display box to record a new hotkey, so that I can customize keyboard shortcuts
5. As a user, I want to see modifier keys (⇧⌥⌘⌃) always displayed in the hotkey box, so that I understand which modifiers are active
6. As a user, I want inactive modifier keys shown in gray and active ones in dark gray, so that I can visually distinguish the hotkey combination
7. As a user, I want the letter key highlighted in blue in the hotkey display, so that I can quickly identify the main key
8. As a user, I want a clear (✕) button next to set hotkeys, so that I can remove a hotkey assignment
9. As a user, I want a "恢复所有默认值" button at the bottom of hotkey pages, so that I can reset all hotkeys in that category
10. As a user, I want a "检测冲突" button at the bottom of hotkey pages, so that I can check if my hotkeys conflict with system shortcuts
11. As a user, I want to switch to the "翻译" tab, so that I can configure translation settings
12. As a user, I want to see translation hotkeys (划词翻译/截图翻译/显示翻译窗口), so that I can customize translation shortcuts
13. As a user, I want to access translation history in the 翻译 tab, so that I can review past translations
14. As a user, I want to favorite translations, so that I can quickly find important translations later
15. As a user, I want to switch to the "OCR" tab, so that I can configure OCR settings
16. As a user, I want to see OCR hotkeys (截图OCR/静默截图OCR/访问选图OCR/显示OCR窗口), so that I can customize OCR shortcuts
17. As a user, I want to access OCR history with thumbnail previews, so that I can review past OCR results
18. As a user, I want to switch to the "服务" tab, so that I can manage all Providers
19. As a user, I want to see top-level tabs in the 服务 tab (OCR服务/翻译服务/语音合成), so that I can navigate between provider types
20. As a user, I want to see OCR Provider cards (Tesseract/System OCR/百度OCR), so that I can view available OCR providers
21. As a user, I want to see the currently active OCR Provider highlighted, so that I understand which one is being used
22. As a user, I want to click "激活" on a Provider card, so that I can enable that provider
23. As a user, I want to click "配置" on a Provider card, so that I can enter API keys and other settings
24. As a user, I want to click "测试" on a Provider card, so that I can verify my configuration works
25. As a user, I want to see translation Provider cards with multiple selection support, so that I can enable multiple translation services simultaneously
26. As a user, I want to drag translation Provider cards to reorder priority, so that I can control which results appear first
27. As a user, I want to see a "已激活：Google ✓ DeepL ✓" indicator, so that I know which translation providers are currently active
28. As a user, I want to see TTS Provider cards in the 语音合成 tab, so that I can configure text-to-speech services
29. As a user, I want to click "+ 添加自定义服务" in translation providers, so that I can add custom OpenAI/Claude/Gemini compatible APIs
30. As a user, I want to switch to the "通用" tab, so that I can configure application-wide settings
31. As a user, I want to change interface language in the 通用 → 界面 page, so that I can use the app in my preferred language
32. As a user, I want to change theme (浅色/深色/跟随系统) in the 通用 → 界面 page, so that I can customize the appearance
33. As a user, I want to toggle "开机自启" in the 通用 → 界面 page, so that the app starts automatically
34. As a user, I want to configure "显示主窗口" and "退出应用" hotkeys in 通用 → 应用快捷键, so that I can control the app itself
35. As a user, I want to see version information in 通用 → 关于, so that I know which version I'm running
36. As a user, I want to click "检查更新" in 通用 → 关于, so that I can update to the latest version
37. As a user, I want to switch to the "高级" tab, so that I can access advanced settings
38. As a user, I want to configure proxy settings in 高级 → 网络, so that the app works behind a corporate firewall
39. As a user, I want to change log level in 高级 → 日志, so that I can troubleshoot issues
40. As a user, I want to click "打开日志目录" in 高级 → 日志, so that I can view log files
41. As a user, I want to click "导出配置" in 高级 → 数据管理, so that I can backup my settings
42. As a user, I want to click "导入配置" in 高级 → 数据管理, so that I can restore settings on a new machine
43. As a user, I want to click "清空历史记录" in 高级 → 数据管理, so that I can free up space
44. As a user, I want to click "清除所有数据" in 高级 → 数据管理 with a confirmation dialog, so that I can reset the app
45. As a user, I want the left navigation to show 6 icon-based buttons (截图/翻译/OCR/服务/通用/高级), so that I can quickly navigate between major sections
46. As a user, I want active navigation buttons highlighted with white background and blue icon, so that I know which section I'm in
47. As a user, I want to hover over navigation buttons to see a semi-transparent white background, so that I get visual feedback
48. As a user, I want icons to use 2px stroke with round line caps, so that they match modern design standards
49. As a user, I want the window to remember its size and position, so that it opens where I left it
50. As a user, I want the window to have a minimum size of 700×500px, so that content remains readable
51. As a user, I want secondary navigation to appear as a left sidebar (224px) in tabs with sub-pages, so that I can navigate within a section
52. As a user, I want the active secondary nav button highlighted in blue with white text, so that I know which sub-page I'm on
53. As a user, I want to search within translation history, so that I can find specific past translations
54. As a user, I want to filter translation history by type (划词/截图/输入), so that I can narrow down results
55. As a user, I want to delete individual history items, so that I can remove unwanted entries
56. As a user, I want to add tags to favorited items, so that I can organize them
57. As a user, I want to add notes to favorited translations, so that I can remember context
58. As a user, I want OCR history to show image thumbnails, so that I can visually identify past OCR operations
59. As a user, I want to click "重新识别" on a favorited OCR item, so that I can re-run OCR with different settings
60. As a user, I want screenshot favorites to show as a grid of thumbnails, so that I can browse them visually
61. As a user, I want unset hotkeys to show "按下快捷键" in a dashed border box, so that I know they need configuration
62. As a user, I want Provider cards to show status badges (已激活/未激活/未配置/内置), so that I understand their current state
63. As a user, I want Provider configuration forms to validate API keys, so that I get immediate feedback
64. As a user, I want Provider test buttons to show loading state, so that I know the test is running
65. As a user, I want test results to show success/failure messages, so that I can diagnose issues
66. As a user, I want "恢复所有默认值" to show a confirmation dialog, so that I don't accidentally reset settings
67. As a user, I want "检测冲突" to show a list of conflicts with suggestions, so that I can resolve them
68. As a user, I want the main content area to scroll independently, so that navigation remains visible
69. As a user, I want form changes to be saved automatically or show a save button, so that my changes persist
70. As a user, I want dangerous actions (清空历史/清除所有数据) to show red buttons with confirmation dialogs, so that I don't lose data accidentally

## Implementation Decisions

### Module Structure

**New Modules to Build**:
- `src/components/SettingsWindow/` - Main settings window container
- `src/components/SettingsWindow/Navigation/MainNav.tsx` - Left sidebar navigation (6 main tabs)
- `src/components/SettingsWindow/Navigation/SecondaryNav.tsx` - Secondary navigation sidebar (for tabs with sub-pages)
- `src/components/SettingsWindow/Icons/` - SVG icon components (already prototyped)
- `src/components/SettingsWindow/Screenshot/` - Screenshot tab pages
- `src/components/SettingsWindow/Translation/` - Translation tab pages
- `src/components/SettingsWindow/OCR/` - OCR tab pages
- `src/components/SettingsWindow/Services/` - Services tab pages
- `src/components/SettingsWindow/General/` - General tab pages
- `src/components/SettingsWindow/Advanced/` - Advanced tab pages
- `src/components/SettingsWindow/Hotkey/HotkeyDisplay.tsx` - Hotkey display component
- `src/components/SettingsWindow/Hotkey/HotkeyRow.tsx` - Hotkey row component
- `src/components/SettingsWindow/Provider/ProviderCard.tsx` - Provider card component
- `src/stores/settingsStore.ts` - Settings state management (Zustand)
- `src/stores/historyStore.ts` - History and favorites state management (Zustand)
- `src/stores/providerStore.ts` - Provider configuration state management (Zustand)

**Modules to Modify**:
- `src/App.tsx` - Replace prototype import with real SettingsWindow
- `src/main.tsx` - Restore normal App import

### State Management

Use Zustand for all settings state:

```typescript
// settingsStore.ts
interface SettingsState {
  // Window state
  activeMainTab: 'screenshot' | 'translation' | 'ocr' | 'services' | 'general' | 'advanced';
  screenshotSubTab: 'hotkeys' | 'save-settings' | 'editor' | 'favorites';
  // ... other sub-tabs
  
  // Hotkeys
  hotkeys: {
    screenshot: HotkeyMap;
    translation: HotkeyMap;
    ocr: HotkeyMap;
    app: HotkeyMap;
  };
  
  // General settings
  language: string;
  theme: 'light' | 'dark' | 'system';
  startOnBoot: boolean;
  
  // Screenshot settings
  screenshotSavePath: string;
  screenshotFormat: 'png' | 'jpg' | 'webp';
  screenshotQuality: number;
  
  // Translation settings
  defaultSourceLang: string;
  defaultTargetLang: string;
  
  // OCR settings
  ocrLanguagePriority: string[];
  
  // Actions
  setActiveMainTab: (tab: string) => void;
  setHotkey: (category: string, key: string, value: string) => void;
  resetHotkeys: (category: string) => void;
  detectConflicts: (category: string) => Conflict[];
  // ... other actions
}
```

```typescript
// providerStore.ts
interface ProviderState {
  ocrProviders: Provider[];
  translationProviders: Provider[];
  ttsProviders: Provider[];
  
  activeOcrProvider: string | null;
  activeTranslationProviders: string[];
  activeTtsProvider: string | null;
  
  activateProvider: (type: 'ocr' | 'translation' | 'tts', id: string) => void;
  deactivateProvider: (type: 'ocr' | 'translation' | 'tts', id: string) => void;
  updateProviderConfig: (id: string, config: any) => void;
  testProvider: (id: string) => Promise<TestResult>;
  reorderTranslationProviders: (ids: string[]) => void;
  // ... other actions
}
```

### Hotkey Display Component

From prototype validation, the hotkey display uses this structure:

```typescript
// Always show all 4 modifier keys (⇧⌥⌘⌃)
// Inactive: text-gray-300, Active: text-gray-700
// Letter key: text-blue-500, text-2xl, font-medium
// Unset: dashed border with "按下快捷键" placeholder

interface HotkeyDisplayProps {
  value: string; // e.g., "⌥S", "⇧⌥S", "未设置"
  onClick?: () => void; // Recording mode
}
```

### Provider Card Component

```typescript
interface ProviderCardProps {
  name: string;
  icon: string;
  status: 'active' | 'inactive' | 'unconfigured' | 'builtin';
  description: string;
  onConfigure: () => void;
  onTest: () => void;
  onActivate?: () => void;
  onDeactivate?: () => void;
  draggable?: boolean; // For translation providers
}
```

### Routing

Use component state (not React Router) for tab navigation, as this is a single settings window:
- `activeMainTab` state controls which main content component renders
- Each tab component manages its own `activeSubTab` state
- No URL changes needed

### Persistence

Settings stored in:
- **Local file**: `~/.snaplingo/config.json` for non-sensitive settings
- **System keychain**: API keys and sensitive data
  - macOS: Keychain
  - Windows: Credential Manager
  - Linux: Secret Service

### Icons

Six SVG icons validated in prototype:
- Screenshot: Camera (path + circle)
- Translation: Globe with meridians
- OCR: Scan frame corners + text lines
- Services: Stacked cards (3 layers)
- General: Toggle sliders
- Advanced: Radial control (center circle + 8-way spokes)

All icons: 24×24px viewBox, 2px stroke, round line caps, `stroke="currentColor"`

### Styling

Use Tailwind classes as in prototype:
- Main nav: w-24 (90px), bg-gradient-to-b from-gray-50 to-gray-100
- Secondary nav: w-56 (224px), bg-gray-50
- Nav buttons: w-16 h-16, rounded-xl
- Active state: bg-white shadow-md text-blue-600
- Hover: bg-white/50
- Content area: flex-1, p-12

### Data Flow

1. User clicks hotkey display → Enter recording mode
2. User presses key combination → Capture and format as "⌥S"
3. Update `settingsStore.hotkeys[category][key]`
4. Register global hotkey listener via Tauri command
5. Persist to config file

Provider activation:
1. User clicks "激活" → Call `activateProvider(type, id)`
2. Update store state
3. If unconfigured, show config modal
4. Persist to config file

## Testing Decisions

### What Makes a Good Test

- Test external behavior (user interactions and visible outcomes), not internal state
- Test through component interfaces, not implementation details
- Mock Tauri commands, not Zustand stores
- Verify DOM output and user-visible state changes

### Modules to Test

**High Priority**:
- `HotkeyDisplay.tsx` - Renders correct modifier key states, letter key highlighting
- `HotkeyRow.tsx` - Click handling, clear button visibility
- `ProviderCard.tsx` - Status badge display, button states
- `MainNav.tsx` - Active state highlighting, click navigation
- `settingsStore.ts` - State updates, hotkey conflict detection
- `providerStore.ts` - Provider activation logic, ordering

**Medium Priority**:
- Secondary navigation components
- Individual settings pages
- Form validation

**Low Priority** (test manually or skip for MVP):
- Visual styling details
- Animation transitions
- Window resize behavior

### Prior Art

Look at existing tests for patterns:
- Component interaction tests (if any exist in `src/components/`)
- Store tests (check if Zustand stores have test files)
- Tauri command mocking patterns (check `src-tauri/` for test examples)

If no prior art exists, follow these patterns:
- Use Vitest + React Testing Library
- Mock Tauri commands with `vi.mock('@tauri-apps/api/...')`
- Test user flows, not isolated units

### Test Examples

```typescript
// HotkeyDisplay.test.tsx
test('displays modifier keys with correct colors', () => {
  render(<HotkeyDisplay value="⌥S" />);
  
  // Shift: inactive (gray)
  expect(screen.getByText('⇧')).toHaveClass('text-gray-300');
  
  // Option: active (dark gray)
  expect(screen.getByText('⌥')).toHaveClass('text-gray-700');
  
  // Command: inactive (gray)
  expect(screen.getByText('⌘')).toHaveClass('text-gray-300');
  
  // Letter: blue
  expect(screen.getByText('S')).toHaveClass('text-blue-500');
});

test('shows dashed border for unset hotkeys', () => {
  const { container } = render(<HotkeyDisplay value="未设置" />);
  expect(container.firstChild).toHaveClass('border-dashed');
  expect(screen.getByText('按下快捷键')).toBeInTheDocument();
});
```

```typescript
// ProviderCard.test.tsx
test('shows activate button for inactive provider', () => {
  const onActivate = vi.fn();
  render(<ProviderCard 
    name="DeepL" 
    status="inactive" 
    description="..." 
    onActivate={onActivate}
  />);
  
  const button = screen.getByText('激活');
  fireEvent.click(button);
  expect(onActivate).toHaveBeenCalled();
});

test('shows status badge', () => {
  render(<ProviderCard name="Google" status="active" description="..." />);
  expect(screen.getByText('已激活')).toHaveClass('bg-green-100', 'text-green-700');
});
```

## Out of Scope

The following are explicitly out of scope for this PRD:

1. **Actual hotkey registration** - This PRD covers the UI only; Tauri backend integration is separate
2. **Provider API implementations** - Only the configuration UI; actual OCR/translation calls are separate
3. **History data persistence** - UI only; backend storage is separate
4. **Screenshot editor** - Separate feature, not part of settings window
5. **Result Window improvements** - Already exists, no changes needed
6. **Tray menu** - Separate feature
7. **First-run onboarding** - Future enhancement
8. **Keyboard navigation** - Accessibility feature for future
9. **Dark mode implementation** - UI only shows the toggle; theme switching is separate
10. **Localization** - UI shows language selector; actual i18n is separate
11. **Update mechanism** - "Check for updates" button UI only
12. **Cloud sync** - No plans for cloud features
13. **Plugin system** - All providers are built-in
14. **Custom hotkey recording UI** - Use simple click-to-record, no fancy modal
15. **Drag-and-drop for provider ordering** - Use up/down buttons initially, drag-drop is enhancement

## Further Notes

### Design Validation

This PRD is based on a fully interactive prototype (`designs/prototypes/App.PROTOTYPE_UI.tsx`) that was validated through browser testing. Key findings:

- 6 main tabs is the right number (not too many, not too few)
- Secondary navigation works well as a left sidebar
- Hotkey display with persistent modifier keys (Bob style) is intuitive
- Global "恢复默认值" and "检测冲突" buttons are cleaner than per-row buttons
- Stacked card icon effectively communicates "multiple providers"

### Implementation Priority

Recommended implementation order:

1. **P0**: Main navigation + one complete tab (e.g., 截图 → 快捷键)
2. **P0**: Hotkey components (HotkeyDisplay, HotkeyRow)
3. **P1**: Settings store + persistence
4. **P1**: Remaining tabs (翻译, OCR)
5. **P1**: Provider store + Provider cards
6. **P1**: Services tab
7. **P2**: History and favorites UI
8. **P2**: General and Advanced tabs
9. **P2**: Form validation and error handling

### Code Reuse from Prototype

The prototype file `designs/prototypes/App.PROTOTYPE_UI.tsx` contains production-ready code for:
- Icon components (copy as-is)
- HotkeyDisplay component structure
- Layout and styling patterns
- Navigation button styles

Extract and refactor these into proper component files rather than rewriting from scratch.

### Anthropic Design Philosophy

Icons and interactions follow Anthropic's design guidelines:
- 2px stroke weight
- Round line caps and joins
- Monochromatic (inherit color)
- Functional clarity over decoration
- Generous spacing with logical groupings

Reference: Research report on Anthropic design philosophy (generated 2026-06-13).
