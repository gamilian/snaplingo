// Compare actual AppKit controls with the existing React views rendered by WebKit.
// Generate fixtures with cargo run --locked -p snaplingo-reference --bin settings_snapshot -- and settings_features_native.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('..', import.meta.url));
const referenceRoot = join(root, 'target/native-settings');
const nativeRoot = join(root, 'target/native-features');
const pages = {
  ocr: ['hotkeys', 'recognition', 'window'],
  translation: ['hotkeys', 'language', 'window', 'behavior'],
};
let scenes = 0;
let rectangles = 0;
let maximumDifference = 0;

function sameCount(actual, expected, label) {
  if (actual.length !== expected.length) {
    throw new Error(`${label}: expected ${expected.length}, got ${actual.length}`);
  }
}

function compare(actual, expected, origin, label) {
  rectangles += 1;
  for (const key of ['x', 'y', 'width', 'height']) {
    const value = expected[key] - (key === 'x' || key === 'y' ? origin[key] : 0);
    const difference = Math.abs(actual[key] - value);
    if (!Number.isFinite(difference) || difference > 0.5) {
      throw new Error(`${label}.${key}: expected ${value}, got ${actual[key]}`);
    }
    maximumDifference = Math.max(maximumDifference, difference);
  }
}

for (const [page, sections] of Object.entries(pages)) {
  for (const theme of ['native', 'studio', 'paper']) {
    for (const mode of ['', '-dark', '-compact', '-dark-compact']) {
    for (const section of sections) {
      const label = `${page}/${theme}${mode}/${section}`;
      const reference = JSON.parse(readFileSync(join(referenceRoot, `reference-${theme}${mode}-${page}-${section}.json`), 'utf8'));
      const native = JSON.parse(readFileSync(join(nativeRoot, `${page}-${theme}${mode}-${section}.json`), 'utf8'));
      sameCount(native.cards, reference.cards, `${label}/cards`);
      sameCount(native.rows, reference.cards, `${label}/row groups`);
      for (let card = 0; card < reference.cards.length; card += 1) {
        const expected = reference.cards[card];
        compare(native.cards[card], expected.frame, reference.content, `${label}/card ${card}`);
        sameCount(native.rows[card], expected.rows, `${label}/card ${card}/rows`);
        for (let row = 0; row < expected.rows.length; row += 1) {
          const controls = native.rows[card][row].controls;
          const expectedControls = expected.rows[row].controls;
          sameCount(controls, expectedControls, `${label}/card ${card}/row ${row}/controls`);
          const text = expected.rows[row].text.split('\n')[0].trim();
          if (native.rows[card][row].text !== text) {
            throw new Error(`${label}/card ${card}/row ${row}: expected label ${text}`);
          }
          for (let control = 0; control < controls.length; control += 1) {
            compare(controls[control], expectedControls[control].frame, reference.content, `${label}/card ${card}/row ${row}/control ${control}`);
          }
        }
      }
      scenes += 1;
    }
    }
  }
}
console.log(`Native Settings layout: ${scenes} reference scenes, ${rectangles} card/control frames, maximum difference ${maximumDifference} pt.`);
