import { beforeEach, describe, expect, it, vi } from 'vitest';

const reactState = vi.hoisted(() => {
  const harness = {
    cursor: 0,
    values: [] as unknown[],
    useState: vi.fn((initialValue: unknown) => {
      const index = harness.cursor;
      harness.cursor += 1;

      if (harness.values.length <= index) {
        harness.values.push(initialValue);
      }

      const setValue = (nextValue: unknown) => {
        harness.values[index] =
          typeof nextValue === 'function'
            ? (nextValue as (previous: unknown) => unknown)(harness.values[index])
            : nextValue;
      };

      return [harness.values[index], setValue] as const;
    }),
  };

  return harness;
});

vi.mock('react', async () => {
  const actual = await vi.importActual<typeof import('react')>('react');
  return {
    ...actual,
    useState: reactState.useState,
  };
});

import type { TranslationPromptStrategy } from '../../../tauri/providers';
import {
  DEFAULT_PROMPT_STRATEGIES,
  DEFAULT_PROMPT_STRATEGY_ID,
  SMART_PROMPT_STRATEGY_ID,
} from './customTranslationProviderFormModel';
import {
  useTranslationPromptStrategyWorkspace,
  type TranslationPromptStrategyWorkspace,
} from './useTranslationPromptStrategyWorkspace';

describe('useTranslationPromptStrategyWorkspace', () => {
  beforeEach(() => {
    reactState.cursor = 0;
    reactState.values.length = 0;
    reactState.useState.mockClear();
  });

  it('loads backend strategies and populates the selected draft', async () => {
    const legal = strategy({ id: 'legal', name: 'Legal', description: 'Terms' });
    const harness = createWorkspaceHarness({
      selectedStrategyId: 'legal',
      listStrategies: vi.fn().mockResolvedValue({ strategies: [generalStrategy(), legal] }),
    });

    await harness.current.loadPromptStrategies('legal');
    harness.render();

    expect(harness.current.promptStrategies).toEqual([generalStrategy(), legal]);
    expect(harness.current.strategyDraftName).toBe('Legal');
    expect(harness.current.strategyDraftDescription).toBe('Terms');
    expect(harness.current.strategyDraftPrompt).toBe('Prompt legal');
  });

  it('falls back to default strategies when loading fails', async () => {
    const harness = createWorkspaceHarness({
      selectedStrategyId: 'missing',
      listStrategies: vi.fn().mockRejectedValue(new Error('offline')),
    });

    await harness.current.loadPromptStrategies('missing');
    harness.render();

    expect(harness.current.promptStrategies).toEqual(DEFAULT_PROMPT_STRATEGIES);
    expect(harness.current.strategyDraftName).toBe('');
    expect(harness.current.strategyDraftDescription).toBe('');
    expect(harness.current.strategyDraftPrompt).toBe('');
  });

  it('clears the draft when selecting smart strategy', () => {
    const harness = createWorkspaceHarness({ selectedStrategyId: 'general' });
    harness.current.setStrategyDraftName('General');
    harness.current.setStrategyDraftDescription('Description');
    harness.current.setStrategyDraftPrompt('Prompt');
    harness.render();

    harness.current.handlePromptStrategyChange(SMART_PROMPT_STRATEGY_ID);
    harness.render();

    expect(harness.selectedStrategyId).toBe(SMART_PROMPT_STRATEGY_ID);
    expect(harness.current.strategyDraftName).toBe('');
    expect(harness.current.strategyDraftDescription).toBe('');
    expect(harness.current.strategyDraftPrompt).toBe('');
  });

  it('saves a selected strategy with trimmed draft fields', async () => {
    const saveStrategies = vi.fn().mockImplementation(async (config) => config);
    const harness = createWorkspaceHarness({
      selectedStrategyId: 'legal',
      saveStrategies,
    });
    const legal = strategy({ id: 'legal', name: 'Legal' });
    await harness.current.loadPromptStrategies('legal', [generalStrategy(), legal]);
    harness.render();
    harness.current.setStrategyDraftName('  Contract  ');
    harness.current.setStrategyDraftDescription('  Terms  ');
    harness.current.setStrategyDraftPrompt('  Translate like counsel  ');
    harness.render();

    await harness.current.handleSaveStrategy();

    expect(saveStrategies).toHaveBeenCalledWith({
      strategies: [
        generalStrategy(),
        {
          ...legal,
          name: 'Contract',
          description: 'Terms',
          system_prompt: 'Translate like counsel',
        },
      ],
    });
  });

  it('adds a custom strategy and selects it', async () => {
    const saveStrategies = vi.fn().mockImplementation(async (config) => config);
    const harness = createWorkspaceHarness({
      selectedStrategyId: DEFAULT_PROMPT_STRATEGY_ID,
      saveStrategies,
      createStrategyId: () => 'custom-fixed',
    });
    harness.current.setStrategyDraftName('  Code  ');
    harness.current.setStrategyDraftDescription('  Dev text  ');
    harness.current.setStrategyDraftPrompt('  Translate code comments  ');
    harness.render();

    await harness.current.handleAddStrategy();
    harness.render();

    expect(saveStrategies).toHaveBeenCalledWith({
      strategies: [
        ...DEFAULT_PROMPT_STRATEGIES,
        {
          id: 'custom-fixed',
          name: 'Code',
          description: 'Dev text',
          system_prompt: 'Translate code comments',
          is_builtin: false,
          is_deletable: true,
        },
      ],
    });
    expect(harness.selectedStrategyId).toBe('custom-fixed');
  });

  it('deletes a deletable strategy and selects general', async () => {
    const saveStrategies = vi.fn().mockImplementation(async (config) => config);
    const custom = strategy({ id: 'custom-code', is_deletable: true });
    const harness = createWorkspaceHarness({
      selectedStrategyId: 'custom-code',
      saveStrategies,
    });
    await harness.current.loadPromptStrategies('custom-code', [
      generalStrategy(),
      custom,
    ]);
    harness.render();

    await harness.current.handleDeleteStrategy();

    expect(saveStrategies).toHaveBeenCalledWith({
      strategies: [generalStrategy()],
    });
    expect(harness.selectedStrategyId).toBe(DEFAULT_PROMPT_STRATEGY_ID);
  });

  it('sets validation and persistence errors', async () => {
    const saveStrategies = vi.fn().mockRejectedValue(new Error('disk full'));
    const harness = createWorkspaceHarness({
      selectedStrategyId: DEFAULT_PROMPT_STRATEGY_ID,
      saveStrategies,
    });

    await harness.current.handleAddStrategy();
    harness.render();
    expect(harness.current.strategyError).toBe('策略名称和系统提示词不能为空');

    harness.current.setStrategyDraftName('General');
    harness.current.setStrategyDraftPrompt('Prompt');
    harness.render();
    await harness.current.handleSaveStrategy();
    harness.render();

    expect(harness.current.strategyError).toBe('保存策略失败: disk full');
  });
});

function createWorkspaceHarness({
  selectedStrategyId,
  listStrategies = vi.fn().mockResolvedValue({ strategies: DEFAULT_PROMPT_STRATEGIES }),
  saveStrategies = vi.fn().mockImplementation(async (config) => config),
  createStrategyId = () => 'custom-test',
}: {
  selectedStrategyId: string;
  listStrategies?: ReturnType<typeof vi.fn>;
  saveStrategies?: ReturnType<typeof vi.fn>;
  createStrategyId?: () => string;
}) {
  let currentSelectedStrategyId = selectedStrategyId;
  let current: TranslationPromptStrategyWorkspace;

  const render = () => {
    reactState.cursor = 0;
    current = useTranslationPromptStrategyWorkspace({
      selectedStrategyId: currentSelectedStrategyId,
      onSelectedStrategyIdChange: (nextStrategyId) => {
        currentSelectedStrategyId = nextStrategyId;
      },
      clients: {
        listTranslationPromptStrategies: listStrategies,
        saveTranslationPromptStrategies: saveStrategies,
      },
      createStrategyId,
    });
  };

  render();

  return {
    get current() {
      return current;
    },
    get selectedStrategyId() {
      return currentSelectedStrategyId;
    },
    render,
  };
}

function generalStrategy(): TranslationPromptStrategy {
  return DEFAULT_PROMPT_STRATEGIES[0];
}

function strategy(
  overrides: Partial<TranslationPromptStrategy> = {},
): TranslationPromptStrategy {
  const id = overrides.id ?? 'custom';
  return {
    id,
    name: `Name ${id}`,
    description: `Description ${id}`,
    system_prompt: `Prompt ${id}`,
    is_builtin: false,
    is_deletable: false,
    ...overrides,
  };
}
