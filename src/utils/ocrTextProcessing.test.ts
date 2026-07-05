import { describe, expect, it } from 'vitest';
import { normalizeOcrText, ocrCopyTokens } from './ocrTextProcessing';

describe('normalizeOcrText', () => {
  it('joins OCR line breaks inside URLs emails phones and English phrases', () => {
    expect(
      normalizeOcrText(
        [
          'Visit https://example.',
          'com/docs?a=1',
          'Email support@',
          'example.com',
          'Phone 138-0013-',
          '8000',
          'Modern Azure Foundry endpoint',
          'URLs. See Azure OpenAI.',
        ].join('\n'),
      ),
    ).toBe(
      [
        'Visit https://example.com/docs?a=1',
        'Email support@example.com',
        'Phone 138-0013-8000',
        'Modern Azure Foundry endpoint URLs. See Azure OpenAI.',
      ].join('\n'),
    );
  });

  it('keeps paragraph breaks and list item boundaries readable', () => {
    expect(
      normalizeOcrText('第一段文字。\n\n第二段文字\n• 列表项一\n- 列表项二'),
    ).toBe('第一段文字。\n\n第二段文字\n• 列表项一\n- 列表项二');
  });
});

describe('ocrCopyTokens', () => {
  it('extracts important copy targets from OCR text in source order', () => {
    expect(
      ocrCopyTokens(
        [
          '官网 https://example.com/docs?a=1, 邮箱 support@example.com',
          '电话 138-0013-8000 或 +1 (650) 253-0000',
          '内网 192.168.1.8:8080，订单号 AB-1234567890，验证码 123456',
        ].join('\n'),
      ),
    ).toEqual([
      {
        id: 'ocr-token-1',
        kind: 'url',
        label: '网址',
        value: 'https://example.com/docs?a=1',
      },
      {
        id: 'ocr-token-2',
        kind: 'email',
        label: '邮箱',
        value: 'support@example.com',
      },
      {
        id: 'ocr-token-3',
        kind: 'phone',
        label: '电话',
        value: '138-0013-8000',
      },
      {
        id: 'ocr-token-4',
        kind: 'phone',
        label: '电话',
        value: '+1 (650) 253-0000',
      },
      {
        id: 'ocr-token-5',
        kind: 'address',
        label: '地址',
        value: '192.168.1.8:8080',
      },
      {
        id: 'ocr-token-6',
        kind: 'code',
        label: '编号',
        value: 'AB-1234567890',
      },
      {
        id: 'ocr-token-7',
        kind: 'code',
        label: '验证码',
        value: '123456',
      },
    ]);
  });

  it('deduplicates repeated values and ignores ordinary prose', () => {
    expect(
      ocrCopyTokens(
        '请访问 www.example.com，备用 www.example.com。普通中文和 short word 不需要切。',
      ),
    ).toEqual([
      {
        id: 'ocr-token-1',
        kind: 'url',
        label: '网址',
        value: 'www.example.com',
      },
    ]);
  });

  it('keeps URL and email tokens intact instead of splitting punctuation', () => {
    expect(
      ocrCopyTokens('联系 a.b-test@example.co.uk 或打开 https://a.b/c-d/e_f?q=x-y.'),
    ).toEqual([
      {
        id: 'ocr-token-1',
        kind: 'email',
        label: '邮箱',
        value: 'a.b-test@example.co.uk',
      },
      {
        id: 'ocr-token-2',
        kind: 'url',
        label: '网址',
        value: 'https://a.b/c-d/e_f?q=x-y',
      },
    ]);
  });

  it('extracts tokens after OCR line-break normalization', () => {
    expect(
      ocrCopyTokens(
        [
          '网址 https://example.',
          'com/docs?a=1',
          '电话 138-0013-',
          '8000',
        ].join('\n'),
      ),
    ).toEqual([
      {
        id: 'ocr-token-1',
        kind: 'url',
        label: '网址',
        value: 'https://example.com/docs?a=1',
      },
      {
        id: 'ocr-token-2',
        kind: 'phone',
        label: '电话',
        value: '138-0013-8000',
      },
    ]);
  });

  it('extracts common daily OCR copy targets beyond contact info', () => {
    expect(
      ocrCopyTokens(
        [
          '验证码 8K2P91，金额 ¥128.00，预约时间 2026-07-05 14:30',
          'Wi-Fi: Cafe_5G 密码: snap-2026',
          '车牌 沪A12345 航班 MU5137 车次 G1234 座位 12A',
          '账号 @snaplingo 坐标 31.2304, 121.4737 身份证 110105199003071234',
        ].join('\n'),
      ).map(({ kind, label, value }) => ({ kind, label, value })),
    ).toEqual([
      { kind: 'code', label: '验证码', value: '8K2P91' },
      { kind: 'money', label: '金额', value: '¥128.00' },
      { kind: 'datetime', label: '时间', value: '2026-07-05 14:30' },
      { kind: 'account', label: '账号', value: 'Cafe_5G' },
      { kind: 'code', label: '密码', value: 'snap-2026' },
      { kind: 'plate', label: '车牌', value: '沪A12345' },
      { kind: 'travel', label: '航班', value: 'MU5137' },
      { kind: 'travel', label: '车次', value: 'G1234' },
      { kind: 'travel', label: '座位', value: '12A' },
      { kind: 'account', label: '账号', value: '@snaplingo' },
      { kind: 'coordinate', label: '坐标', value: '31.2304, 121.4737' },
      { kind: 'sensitive', label: '敏感信息', value: '110105199003071234' },
    ]);
  });

  it('uses stricter link and phone extraction for fuzzy domains and spaced phone numbers', () => {
    expect(
      ocrCopyTokens('访问 example.com，联系电话 138 0013 8000，错误短号 12345'),
    ).toEqual([
      {
        id: 'ocr-token-1',
        kind: 'url',
        label: '网址',
        value: 'example.com',
      },
      {
        id: 'ocr-token-2',
        kind: 'phone',
        label: '电话',
        value: '138 0013 8000',
      },
    ]);
  });
});
