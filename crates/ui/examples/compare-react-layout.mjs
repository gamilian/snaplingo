// Run layout_report and the existing settings_reference_snapshot harness first.
import { readFileSync } from 'node:fs';
const referenceRoot = process.argv[2] ?? 'target/native-settings';
const sharedRoot = process.argv[3] ?? 'target/shared-ui';
let count = 0;
let maximum = 0;
const differences = [];
let scenes=0;
for (const [page,sections] of [['ocr',['hotkeys','recognition','window']],['translation',['hotkeys','language','window','behavior']]]) {
for (const theme of ['native', 'studio', 'paper']) {
  for (const mode of ['', '-dark', '-compact', '-dark-compact']) {
    for (const section of sections) {
      scenes+=1;
      const name = `${theme}${mode}-${page}-${section}`;
      const reference = JSON.parse(readFileSync(`${referenceRoot}/reference-${name}.json`, 'utf8'));
      const shared = JSON.parse(readFileSync(`${sharedRoot}/${page}-${theme}${mode}-${section}.json`, 'utf8'));
      const compare = (actual, expected, label) => {
        count += 1;
        for (const key of ['x', 'y', 'width', 'height']) {
          const error = Math.abs(actual[key] - expected[key]);
          maximum = Math.max(maximum, error);
          if (error > 0.5) differences.push(`${name}/${label}.${key}: expected ${expected[key]}, got ${actual[key]} (Δ${error})`);
        }
      };
      compare(shared.content, reference.content, 'content');
      if (shared.cards.length !== reference.cards.length) throw new Error(`${name}: card count differs`);
      for (let card = 0; card < reference.cards.length; card += 1) {
        const actual = shared.cards[card];
        const expected = reference.cards[card];
        compare(actual.frame, expected.frame, `card${card}`);
        if (actual.rows.length !== expected.rows.length) throw new Error(`${name}/card${card}: row count differs`);
        for (let row = 0; row < expected.rows.length; row += 1) {
          if (actual.rows[row].text !== expected.rows[row].text.split('\n')[0].trim()) throw new Error(`${name}/card${card}/row${row}: label differs`);
          if (actual.rows[row].controls.length !== expected.rows[row].controls.length) throw new Error(`${name}/card${card}/row${row}: controls differ`);
          for (let control = 0; control < expected.rows[row].controls.length; control += 1) compare(actual.rows[row].controls[control], expected.rows[row].controls[control].frame, `card${card}/row${row}/control${control}`);
        }
      }
    }
  }
}
}
console.log(`Compared ${scenes} actual React scenes, ${count} shared card/control frames; maximum difference ${maximum} DIP.`);
if (differences.length) { console.error(differences.slice(0, 40).join('\n')); process.exitCode = 1; }
