import LinkifyIt from 'linkify-it';
import { searchPhoneNumbersInText } from 'libphonenumber-js/min';

const listItemPattern = /^\s*(?:[-*•·]|\d+[.)、])\s+/;
const labeledFieldPattern =
  /^\s*(?:Email|E-mail|Phone|Tel|Mobile|URL|网址|邮箱|电话|手机|座机|验证码|订单号)(?:\b|\s|$)/i;
const noSpaceJoinTailPattern = /(?:[@/._~:?#&=%+-]|https?:\/\/|www\.)$/i;
const terminalSentencePattern = /[。！？!?；;]$/;

export type OcrCopyTokenKind =
  | 'url'
  | 'email'
  | 'phone'
  | 'address'
  | 'code'
  | 'number'
  | 'datetime'
  | 'money'
  | 'account'
  | 'plate'
  | 'travel'
  | 'coordinate'
  | 'sensitive';

export interface OcrCopyToken {
  id: string;
  kind: OcrCopyTokenKind;
  label: string;
  value: string;
}

interface TokenCandidate {
  kind: OcrCopyTokenKind;
  label: string;
  value: string;
  start: number;
  end: number;
  priority: number;
}

const tokenPatterns: Array<{
  kind: OcrCopyTokenKind;
  label: string;
  priority: number;
  pattern: RegExp;
  accept?: (value: string) => boolean;
}> = [
  {
    kind: 'sensitive',
    label: '敏感信息',
    priority: 95,
    pattern: /\b\d{17}[\dXx]\b/g,
  },
  {
    kind: 'coordinate',
    label: '坐标',
    priority: 92,
    pattern: /\b-?\d{1,2}\.\d{4,}\s*,\s*-?\d{1,3}\.\d{4,}\b/g,
  },
  {
    kind: 'datetime',
    label: '时间',
    priority: 90,
    pattern:
      /\b\d{4}[-/.]\d{1,2}[-/.]\d{1,2}(?:\s+\d{1,2}:\d{2}(?::\d{2})?)?\b/g,
  },
  {
    kind: 'money',
    label: '金额',
    priority: 88,
    pattern: /(?:[¥￥$€£]\s*\d+(?:,\d{3})*(?:\.\d{1,2})?|\b\d+(?:,\d{3})*(?:\.\d{1,2})?\s*(?:元|人民币|USD|CNY|EUR|GBP)\b)/gi,
  },
  {
    kind: 'address',
    label: '地址',
    priority: 80,
    pattern:
      /\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)(?::\d{2,5})?\b/g,
  },
  {
    kind: 'plate',
    label: '车牌',
    priority: 78,
    pattern: /[\u4e00-\u9fa5][A-Z][A-Z0-9]{5,6}\b/g,
  },
  {
    kind: 'travel',
    label: '航班',
    priority: 76,
    pattern: /(?<=(?:航班|Flight)\s*)[A-Z]{2}\d{3,4}\b/gi,
  },
  {
    kind: 'travel',
    label: '车次',
    priority: 75,
    pattern: /(?<=(?:车次|Train)\s*)[GDCZTKY]\d{1,4}\b/gi,
  },
  {
    kind: 'travel',
    label: '座位',
    priority: 74,
    pattern: /(?<=(?:座位|Seat)\s*)\d{1,2}[A-Z]\b/gi,
  },
  {
    kind: 'account',
    label: '账号',
    priority: 72,
    pattern: /@[A-Z0-9_][A-Z0-9_.-]{2,}/gi,
  },
  {
    kind: 'account',
    label: '账号',
    priority: 71,
    pattern: /(?<=(?:Wi-?Fi|SSID|账号|用户名|User(?:name)?)[:：]\s*)[A-Z0-9_.-]{3,}/gi,
  },
  {
    kind: 'code',
    label: '验证码',
    priority: 70,
    pattern: /(?<=(?:验证码|校验码|Code|OTP)[:：]?\s*)[A-Z0-9]{4,8}\b/gi,
  },
  {
    kind: 'code',
    label: '密码',
    priority: 69,
    pattern: /(?<=(?:密码|Password|Passcode)[:：]?\s*)[A-Z0-9_.@#$%&*!-]{4,}\b/gi,
  },
  {
    kind: 'code',
    label: '编号',
    priority: 60,
    pattern: /\b[A-Z]{1,8}[-_]\d[A-Z0-9_-]{5,}\b/gi,
  },
  {
    kind: 'code',
    label: '编号',
    priority: 55,
    pattern: /\b[0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12}\b/gi,
  },
  {
    kind: 'code',
    label: '编号',
    priority: 50,
    pattern: /\b(?=[A-Z0-9_-]{10,}\b)(?=[A-Z0-9_-]*\d)[A-Z0-9_-]+\b/gi,
  },
  {
    kind: 'number',
    label: '数字',
    priority: 40,
    pattern: /\b\d{6,}\b/g,
  },
];

export function normalizeOcrText(text: string) {
  const lines = text
    .replace(/\r\n?/g, '\n')
    .split('\n')
    .map((line) => line.trim());
  const normalizedLines: string[] = [];

  for (const line of lines) {
    if (!line) {
      if (normalizedLines[normalizedLines.length - 1] !== '') {
        normalizedLines.push('');
      }
      continue;
    }

    const previous = normalizedLines[normalizedLines.length - 1];
    if (previous && shouldJoinOcrLines(previous, line)) {
      normalizedLines[normalizedLines.length - 1] =
        previous + lineJoinSeparator(previous, line) + line;
    } else {
      normalizedLines.push(line);
    }
  }

  return normalizedLines.join('\n').trim();
}

export function applyOcrTextPreferences(
  text: string,
  preferences: {
    preserveFormatting: boolean;
    removeChineseSpaces: boolean;
  },
) {
  let result = preferences.preserveFormatting
    ? text.replace(/\r\n?/g, '\n').trim()
    : normalizeOcrText(text);

  if (preferences.removeChineseSpaces) {
    result = result.replace(
      /([\p{Script=Han}])[\t \u00a0]+(?=[\p{Script=Han}])/gu,
      '$1',
    );
  }

  if (!preferences.preserveFormatting) {
    result = result.replace(/\s+/g, ' ').trim();
  }

  return result;
}

export function ocrCopyTokens(text: string): OcrCopyToken[] {
  const normalizedText = normalizeOcrText(text);
  const candidates = [
    ...linkCandidates(normalizedText),
    ...phoneCandidates(normalizedText),
    ...ruleCandidates(normalizedText),
  ];

  const accepted = nonOverlappingCandidates(candidates);
  const seen = new Set<string>();

  return accepted.flatMap((candidate) => {
    const dedupeKey = `${candidate.kind}:${candidate.value}`;
    if (seen.has(dedupeKey)) return [];

    seen.add(dedupeKey);
    return [
      {
        id: `ocr-token-${seen.size}`,
        kind: candidate.kind,
        label: candidate.label,
        value: candidate.value,
      },
    ];
  });
}

const linkify = new LinkifyIt();

function linkCandidates(text: string): TokenCandidate[] {
  return (linkify.match(text) ?? []).map((match) => {
    const isEmail = match.schema === 'mailto:';

    return {
      kind: isEmail ? 'email' : 'url',
      label: isEmail ? '邮箱' : '网址',
      value: normalizeTokenValue(match.text),
      start: match.index,
      end: match.lastIndex,
      priority: isEmail ? 100 : 98,
    };
  });
}

function phoneCandidates(text: string): TokenCandidate[] {
  return Array.from(searchPhoneNumbersInText(text, 'CN')).flatMap((match) => {
    const value = normalizeTokenValue(text.slice(match.startsAt, match.endsAt));
    if (isLikelyIpAddress(value)) return [];

    return [
      {
        kind: 'phone' as const,
        label: '电话',
        value,
        start: match.startsAt,
        end: match.endsAt,
        priority: 94,
      },
    ];
  });
}

function isLikelyIpAddress(value: string) {
  return /^(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)$/.test(
    value,
  );
}

function ruleCandidates(text: string): TokenCandidate[] {
  return tokenPatterns.flatMap(({ pattern, accept, ...token }) =>
    matchesForPattern(text, pattern).flatMap((match) => {
      const normalized = normalizeTokenValue(match.value);
      if (!normalized || accept?.(normalized) === false) return [];

      return [
        {
          ...token,
          value: normalized,
          start: match.start,
          end: match.end,
        },
      ];
    }),
  );
}

function shouldJoinOcrLines(previous: string, next: string) {
  if (listItemPattern.test(next)) return false;
  if (isTokenContinuation(previous, next)) return true;
  if (
    endsWithUsefulToken(previous) &&
    (labeledFieldPattern.test(previous) || labeledFieldPattern.test(next))
  ) {
    return false;
  }
  if (terminalSentencePattern.test(previous)) return false;

  return true;
}

function isTokenContinuation(previous: string, next: string) {
  return (
    noSpaceJoinTailPattern.test(previous) ||
    (previous.includes('@') && !previous.includes(' ')) ||
    (previous.endsWith('-') && /^\d/.test(next))
  );
}

function endsWithUsefulToken(text: string) {
  return (
    /(?:https?:\/\/|www\.)\S+$/i.test(text) ||
    /[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$/i.test(text) ||
    /\d[\d\s().-]{6,}\d$/.test(text)
  );
}

function lineJoinSeparator(previous: string, next: string) {
  if (isTokenContinuation(previous, next)) return '';
  if (
    isCjk(previous[previous.length - 1]) ||
    isCjk(next[0])
  ) {
    return '';
  }

  return ' ';
}

function matchesForPattern(text: string, pattern: RegExp) {
  return Array.from(text.matchAll(pattern), (match) => ({
    value: match[0],
    start: match.index ?? 0,
    end: (match.index ?? 0) + match[0].length,
  }));
}

function normalizeTokenValue(value: string) {
  return value
    .trim()
    .replace(/^[([{<"'“‘]+/, '')
    .replace(/[)\]}>,"'“”‘’。！？；：、，.]+$/u, '');
}

function nonOverlappingCandidates(candidates: TokenCandidate[]) {
  const accepted: TokenCandidate[] = [];

  for (const candidate of candidates.sort(compareCandidates)) {
    if (
      accepted.some(
        (existing) =>
          candidate.start < existing.end && candidate.end > existing.start,
      )
    ) {
      continue;
    }

    accepted.push(candidate);
  }

  return accepted.sort((a, b) => a.start - b.start || b.priority - a.priority);
}

function compareCandidates(a: TokenCandidate, b: TokenCandidate) {
  return a.start - b.start || b.priority - a.priority || b.end - a.end;
}

function isCjk(char: string | undefined) {
  return Boolean(char && /[\u3400-\u9fff]/u.test(char));
}
