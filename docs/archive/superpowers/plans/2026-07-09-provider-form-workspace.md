# Provider Form Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deepen the Custom Translation Provider dialog seam so provider form rules, prompt strategy editing, provider introspection, and rendering are locally testable without changing user-visible behavior.

**Architecture:** Keep `CustomTranslationProviderDialog.tsx` as the composition shell for the custom provider editor. Add focused modules around it: a pure form model, a prompt strategy workspace hook, provider introspection actions, and a render-only view. Preserve the existing frontend Provider Tauri Adapter seam in `src/tauri/providers.ts`; this plan does not move backend Provider Configuration or command behavior.

**Tech Stack:** React, TypeScript, Vitest, Tauri frontend adapters.

---

## Scope

In scope:
- `CustomTranslationProviderDialog.tsx` thinning
- provider form default/trim/request construction locality
- prompt strategy load/add/save/delete locality
- model-list and provider-test action locality
- render-only view extraction for the dialog markup
- focused tests for the new module interfaces

Out of scope:
- backend Provider Configuration changes
- backend command or LLM protocol refactors
- visual redesign of the dialog
- changing labels, defaults, protocol behavior, prompt strategy persistence, or model testing behavior
- changing `TranslationProvidersPage` store behavior

## File Structure

Create:
- `src/components/SettingsWindow/Services/customTranslationProviderFormModel.ts`
  - Pure form model helpers for protocol defaults, initial values, save eligibility, request building, and formatting save errors.
- `src/components/SettingsWindow/Services/customTranslationProviderFormModel.test.ts`
  - Tests add/edit defaults, protocol transitions, save eligibility, and add/update request payload construction.
- `src/components/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.ts`
  - Hook that owns prompt strategy state, draft state, load, select, save, add, delete, and persistence error handling.
- `src/components/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.test.tsx`
  - Hook-level tests using a lightweight React hook harness and injected prompt strategy clients.
- `src/components/SettingsWindow/Services/customProviderIntrospection.ts`
  - Pure async action helpers for list-models and test-connection workflow with injected clients.
- `src/components/SettingsWindow/Services/customProviderIntrospection.test.ts`
  - Tests protocol routing, empty model list error, failure formatting, and test-connection status.
- `src/components/SettingsWindow/Services/CustomTranslationProviderDialogView.tsx`
  - Render-only module for the JSX currently in `CustomTranslationProviderDialog.tsx`.

Modify:
- `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx`
  - Compose the form model, prompt strategy hook, provider introspection actions, and view.
- `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.test.tsx`
  - Keep existing behavior tests green; adjust imports only if needed.
- `src/components/SettingsWindow/Services/customTranslationProviderForm.ts`
  - Keep endpoint preview helpers as-is unless type reuse is needed.

Docs:
- Modify `CONTEXT.md` only if a new domain term is introduced. Expected: no docs change; "Provider Configuration Module" and "Frontend Tauri Adapter" already cover this seam.

## Task 1: Extract the pure custom provider form model

**Files:**
- Create: `src/components/SettingsWindow/Services/customTranslationProviderFormModel.ts`
- Create: `src/components/SettingsWindow/Services/customTranslationProviderFormModel.test.ts`
- Modify: `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx`

- [ ] **Step 1: Write failing form model tests**

Cover:
- initial add defaults match the current dialog defaults
- initial edit values preserve provider protocol, endpoint, model, reasoning level, and prompt strategy
- OpenAI Responses belongs to the OpenAI protocol family
- protocol family changes produce the same endpoint/model defaults currently used by the dialog
- add request trims name, endpoint, model, and api key
- edit request omits blank api key but preserves prompt strategy and fallback strategy
- save eligibility matches existing add/edit behavior

- [ ] **Step 2: Run the focused tests to verify RED**

Run:

```bash
npm test -- customTranslationProviderFormModel.test.ts
```

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement `customTranslationProviderFormModel.ts`**

Export:

```ts
export type LLMProtocol = 'openai' | 'openai-responses' | 'anthropic' | 'gemini';
export type LLMProtocolFamily = 'openai' | 'anthropic' | 'gemini';
export const SMART_PROMPT_STRATEGY_ID = 'smart';
export const DEFAULT_PROMPT_STRATEGY_ID = 'general';
export const PROTOCOL_OPTIONS: Array<{ value: LLMProtocolFamily; label: string }>;
export const OPENAI_MODE_OPTIONS: Array<{ value: LLMProtocol; label: string }>;
export const REASONING_OPTIONS: Array<{ value: string; label: string }>;
export function getInitialCustomProviderFormValues(provider: Provider | null): CustomProviderFormValues;
export function getProtocolDefaults(protocol: LLMProtocol): { endpoint: string; model: string };
export function getProtocolFamily(protocol: LLMProtocol): LLMProtocolFamily;
export function isLLMProtocol(value: string | undefined): value is LLMProtocol;
export function canSaveCustomProviderForm(input: SaveEligibilityInput): boolean;
export function buildAddCustomProviderRequest(input: CustomProviderFormInput): AddCustomTranslationProviderRequest | null;
export function buildUpdateCustomProviderRequest(input: CustomProviderFormInput): UpdateCustomTranslationProviderRequest | null;
export function formatCustomProviderError(error: unknown): string;
```

Do not import React or Tauri adapters in this pure module.

- [ ] **Step 4: Rewire dialog form model usage**

In `CustomTranslationProviderDialog.tsx`:
- import the constants/types/helpers from `customTranslationProviderFormModel.ts`
- remove duplicate local constants and helper functions
- keep state ownership and JSX in the dialog for this task

- [ ] **Step 5: Verify focused behavior**

Run:

```bash
npm test -- customTranslationProviderFormModel.test.ts CustomTranslationProviderDialog.test.tsx customTranslationProviderForm.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/SettingsWindow/Services/customTranslationProviderFormModel.ts src/components/SettingsWindow/Services/customTranslationProviderFormModel.test.ts src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx
git commit -m "refactor(providers): extract custom provider form model"
```

## Task 2: Extract prompt strategy workspace state

**Files:**
- Create: `src/components/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.ts`
- Create: `src/components/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.test.tsx`
- Modify: `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx`

- [ ] **Step 1: Write failing prompt strategy workspace tests**

Use injected clients instead of direct Tauri calls. Cover:
- load uses backend strategies and populates the selected draft
- load failure falls back to default strategies and clears unavailable draft
- selecting `smart` clears the draft
- saving a selected strategy trims name/description/prompt and persists the full strategy list
- adding a strategy creates a deletable custom strategy and selects it
- deleting a deletable strategy selects `general`
- blank name or prompt sets `策略名称和系统提示词不能为空`
- persistence failures set `保存策略失败: <message>`

- [ ] **Step 2: Run focused tests to verify RED**

Run:

```bash
npm test -- useTranslationPromptStrategyWorkspace.test.tsx
```

Expected: FAIL because the hook does not exist.

- [ ] **Step 3: Implement `useTranslationPromptStrategyWorkspace.ts`**

Export:

```ts
export interface TranslationPromptStrategyClients {
  listTranslationPromptStrategies: typeof listTranslationPromptStrategies;
  saveTranslationPromptStrategies: typeof saveTranslationPromptStrategies;
}

export function useTranslationPromptStrategyWorkspace(options: {
  selectedStrategyId: string;
  onSelectedStrategyIdChange(strategyId: string): void;
  clients?: TranslationPromptStrategyClients;
}): TranslationPromptStrategyWorkspace;
```

The hook owns:
- `promptStrategies`
- strategy draft name/description/prompt
- `strategyError`
- `loadPromptStrategies`
- `handlePromptStrategyChange`
- `handleSaveStrategy`
- `handleAddStrategy`
- `handleDeleteStrategy`
- draft setters

Do not render JSX in this hook. Keep default strategies in the form model or export them from this hook if that keeps locality better.

- [ ] **Step 4: Rewire dialog prompt strategy logic**

In `CustomTranslationProviderDialog.tsx`:
- remove prompt strategy `useState` cluster and local load/persist helpers
- call `useTranslationPromptStrategyWorkspace(...)`
- pass hook state/actions to the existing JSX
- preserve the current `useEffect` behavior that loads strategies on open

- [ ] **Step 5: Verify focused behavior**

Run:

```bash
npm test -- useTranslationPromptStrategyWorkspace.test.tsx CustomTranslationProviderDialog.test.tsx customTranslationProviderFormModel.test.ts
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.ts src/components/SettingsWindow/Services/useTranslationPromptStrategyWorkspace.test.tsx src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx
git commit -m "refactor(providers): extract prompt strategy workspace"
```

## Task 3: Extract provider introspection actions

**Files:**
- Create: `src/components/SettingsWindow/Services/customProviderIntrospection.ts`
- Create: `src/components/SettingsWindow/Services/customProviderIntrospection.test.ts`
- Modify: `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx`

- [ ] **Step 1: Write failing introspection action tests**

Cover:
- OpenAI and OpenAI Responses model listing calls `listOpenAICompatibleModels`
- Anthropic model listing calls `listAnthropicModels`
- Gemini model listing calls `listGeminiModels`
- empty model list returns `未返回可用模型`
- list failure clears models and formats `获取模型失败: <message>`
- protocol-specific test action calls the matching test function
- test success returns `{ type: "success", message: "检测成功" }`
- test failure returns `{ type: "error", message: "检测失败: <message>" }`

- [ ] **Step 2: Run focused tests to verify RED**

Run:

```bash
npm test -- customProviderIntrospection.test.ts
```

Expected: FAIL because the module does not exist.

- [ ] **Step 3: Implement `customProviderIntrospection.ts`**

Export:

```ts
export interface CustomProviderIntrospectionClients {
  listOpenAICompatibleModels: typeof listOpenAICompatibleModels;
  listAnthropicModels: typeof listAnthropicModels;
  listGeminiModels: typeof listGeminiModels;
  testOpenAICompatibleProvider: typeof testOpenAICompatibleProvider;
  testOpenAIResponsesProvider: typeof testOpenAIResponsesProvider;
  testAnthropicProvider: typeof testAnthropicProvider;
  testGeminiProvider: typeof testGeminiProvider;
}

export async function loadCustomProviderModels(input: LoadCustomProviderModelsInput): Promise<LoadCustomProviderModelsResult>;
export async function testCustomProviderConnection(input: TestCustomProviderConnectionInput): Promise<TestCustomProviderConnectionResult>;
```

Keep this module React-free. Use `formatCustomProviderError` from the form model.

- [ ] **Step 4: Rewire dialog introspection logic**

In `CustomTranslationProviderDialog.tsx`:
- keep `models`, `isLoadingModels`, `modelListError`, `isTestingProvider`, and `testStatus` state in the composition shell
- replace protocol-specific branching in handlers with calls to the new action helpers
- remove direct imports of model/test Tauri adapter functions from the dialog

- [ ] **Step 5: Verify focused behavior**

Run:

```bash
npm test -- customProviderIntrospection.test.ts CustomTranslationProviderDialog.test.tsx customTranslationProviderFormModel.test.ts useTranslationPromptStrategyWorkspace.test.tsx
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/SettingsWindow/Services/customProviderIntrospection.ts src/components/SettingsWindow/Services/customProviderIntrospection.test.ts src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx
git commit -m "refactor(providers): extract provider introspection actions"
```

## Task 4: Split the render-only dialog view

**Files:**
- Create: `src/components/SettingsWindow/Services/CustomTranslationProviderDialogView.tsx`
- Modify: `src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx`
- Modify tests only if import paths or component names need updates

- [ ] **Step 1: Add or sharpen view smoke coverage**

Prefer keeping existing behavior tests in `CustomTranslationProviderDialog.test.tsx` instead of adding brittle snapshots. If a stable assertion is needed, add one that verifies:
- inline presentation does not use a portal
- dialog presentation still uses `document.body` portal
- rendered controls still contain protocol, model list, strategy editor, cancel, and save controls

- [ ] **Step 2: Run focused tests before extraction**

Run:

```bash
npm test -- CustomTranslationProviderDialog.test.tsx
```

Expected: PASS before moving JSX.

- [ ] **Step 3: Move JSX into `CustomTranslationProviderDialogView.tsx`**

The view receives props for:
- dialog title and presentation
- basic form values and setters/handlers
- protocol/reasoning option metadata
- endpoint preview
- model list state and handlers
- test status state and handler
- prompt strategy state/draft/actions
- save/cancel state and handlers

The view must not:
- import Tauri adapters
- call `useState`/`useEffect`
- know how add/update requests are built
- load or persist prompt strategies
- list models or test providers

- [ ] **Step 4: Reduce `CustomTranslationProviderDialog.tsx` to composition**

After this task, `CustomTranslationProviderDialog.tsx` should mostly:
- initialize form state
- reset/load workspace state when opened
- build add/update request through form model helpers
- bind prompt strategy workspace
- bind provider introspection actions
- render `CustomTranslationProviderDialogView`
- wrap portal behavior if `presentation === "dialog"`

- [ ] **Step 5: Verify focused frontend behavior**

Run:

```bash
npm test -- CustomTranslationProviderDialog.test.tsx customTranslationProviderFormModel.test.ts useTranslationPromptStrategyWorkspace.test.tsx customProviderIntrospection.test.ts customTranslationProviderForm.test.ts providerConfigPages.test.tsx TranslationProvidersPage.test.tsx
npm run build
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/SettingsWindow/Services/CustomTranslationProviderDialogView.tsx src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx src/components/SettingsWindow/Services/CustomTranslationProviderDialog.test.tsx
git commit -m "refactor(providers): split custom provider dialog view"
```

## Task 5: Residue check, documentation decision, and full verification

**Files:**
- Modify: `CONTEXT.md` only if the implementation introduces a new named module concept.

- [ ] **Step 1: Run architecture residue searches**

Run:

```bash
wc -l src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx
rg "listOpenAICompatibleModels|testOpenAICompatibleProvider|saveTranslationPromptStrategies|listTranslationPromptStrategies" src/components/SettingsWindow/Services/CustomTranslationProviderDialog.tsx
rg "useState|useEffect|createPortal" src/components/SettingsWindow/Services/CustomTranslationProviderDialogView.tsx
```

Expected:
- `CustomTranslationProviderDialog.tsx` is materially smaller than 814 lines
- direct model/test/prompt strategy adapter imports are gone from the dialog
- `CustomTranslationProviderDialogView.tsx` has no state/effect/portal ownership

- [ ] **Step 2: Decide whether docs need an update**

If a new domain term such as "Provider Form Workspace" is introduced into production naming and useful for future architecture reviews, update `CONTEXT.md` under Provider Configuration / Frontend Tauri Adapter. If the modules are implementation-detail helpers only, do not change docs.

- [ ] **Step 3: Run full verification**

Run:

```bash
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --tests
cargo fmt --manifest-path src-tauri/Cargo.toml --check
git diff --check
```

Expected: PASS.

- [ ] **Step 4: Commit documentation or residue-only cleanup if needed**

If docs or cleanup changed:

```bash
git add CONTEXT.md src/components/SettingsWindow/Services
git commit -m "docs(providers): document custom provider form seam"
```

If no docs/cleanup changed, do not create an empty commit.

## Notes

- Keep the existing Chinese UI copy unchanged.
- Keep protocol defaults unchanged:
  - OpenAI: `https://api.openai.com`, `gpt-4o`
  - OpenAI Responses: `https://api.openai.com`, `gpt-5-mini`
  - Anthropic: `https://api.anthropic.com`, `claude-3-5-sonnet-latest`
  - Gemini: `https://generativelanguage.googleapis.com`, `gemini-1.5-flash`
- Keep smart strategy fallback as `general`; do not reintroduce a fallback selector.
- Do not move backend Provider Configuration or command logic in this plan.
- Do not add generic form frameworks or new dependencies.
- Do not visually redesign the dialog. This is a seam deepening refactor.
