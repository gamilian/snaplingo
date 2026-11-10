import { renderToStaticMarkup } from 'react-dom/server';
import { describe, expect, it, vi } from 'vitest';
import { CustomSelect } from './CustomSelect';

describe('CustomSelect', () => {
  it('can render a different selected label from the menu option label', () => {
    const markup = renderToStaticMarkup(
      <CustomSelect
        options={[
          {
            value: 'zh-CN',
            label: '中文简体 Chinese (Simplified)',
          },
        ]}
        value="zh-CN"
        selectedLabel="中文简体"
        onChange={vi.fn()}
        align="center"
      />,
    );

    expect(markup).toContain('中文简体');
    expect(markup).not.toContain('中文简体 Chinese (Simplified)');
  });
});
