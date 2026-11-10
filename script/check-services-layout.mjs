// Compare isolated shared Rust Services with actual retained React/WebKit views.
// Generate references: cargo run --locked -p snaplingo-reference --bin settings_snapshot -- services [dark] [compact].
// Generate shared frames: cargo run -p snaplingo-ui --features rendering --example services_snapshot.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('..', import.meta.url));
let scenes = 0;
let rectangles = 0;
let maximumDifference = 0;

function compare(actual, expected, label) {
  rectangles += 1;
  for (const key of ['x', 'y', 'width', 'height']) {
    const difference = Math.abs(actual[key] - expected[key]);
    if (!Number.isFinite(difference) || difference > 0.5) {
      throw new Error(`${label}.${key}: expected ${expected[key]}, got ${actual[key]}`);
    }
    maximumDifference = Math.max(maximumDifference, difference);
  }
}

for (const theme of ['native', 'studio', 'paper']) {
  for (const mode of ['', '-dark', '-compact', '-dark-compact']) {
    for (const scenario of ['ocr', 'translation', 'tts', 'ocr-credentials', 'translation-credentials', 'deeplx', 'deepl', 'custom', 'custom-prompt']) {
      const name = `${theme}${mode}-${scenario}`;
      const reference = JSON.parse(readFileSync(join(root, `target/native-settings/reference-services-${name}.json`), 'utf8'));
      const shared = JSON.parse(readFileSync(join(root, `target/shared-services/${name}.json`), 'utf8'));
      const controls = shared.controls.filter(control => !control.target.startsWith('Shell'));
      compare(shared.content, reference.content, `${name}/content`);
      for (const expected of reference.controls) {
        if (expected.frame.width === 0) continue;
        let label = expected.label?.trim() || expected.placeholder || (expected.tag === 'INPUT' ? '语速' : '标准 DeepL');
        if (label === '返回供应商列表') label = '‹';
        if (label === '测试联通') label = '测试';
        const index = controls.findIndex(control => control.label === label || (expected.placeholder && control.label === expected.placeholder) || (label.startsWith('婷婷') && control.target === 'VoiceMenu'));
        if (index < 0) throw new Error(`${name}: missing ${label}`);
        compare(controls.splice(index, 1)[0].frame, expected.frame, `${name}/${label}`);
      }
      for (const provider of reference.providers) {
        const nearest = shared.fills.reduce((nearest, candidate) => {
          const distance = frame => ['x', 'y', 'width', 'height'].reduce((sum, key) => sum + Math.abs(frame[key] - provider.frame[key]), 0);
          return distance(candidate) < distance(nearest) ? candidate : nearest;
        });
        compare(nearest, provider.frame, `${name}/${provider.name}/card`);
      }
      scenes += 1;
    }
  }
}
console.log(`Shared Services layout: ${scenes} real React scenes, ${rectangles} content/card/control frames, maximum difference ${maximumDifference} pt.`);
