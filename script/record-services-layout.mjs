// Persist only real DOM geometry; this script never reads Rust renderer output.
// Run the four `settings_reference_snapshot services [dark] [compact]` variants first.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';

const root = fileURLToPath(new URL('..', import.meta.url));
const scenarios = ['ocr', 'translation', 'tts', 'ocr-credentials', 'translation-credentials', 'deeplx', 'deepl', 'custom', 'custom-prompt'];
const rect = value => ['x', 'y', 'width', 'height'].map(key => value[key]);
const color = value => [...value.match(/\d+/g).slice(0, 3).map(Number), 255];
const scenes = [];
for (const theme of ['native', 'studio', 'paper']) {
  for (const dark of [false, true]) {
    for (const compact of [false, true]) {
      for (const scenario of scenarios) {
        const source = `reference-services-${theme}${dark ? '-dark' : ''}${compact ? '-compact' : ''}-${scenario}.json`;
        const reference = JSON.parse(readFileSync(join(root, 'target/native-settings', source), 'utf8'));
        scenes.push({ source, theme, dark, width: compact ? 700 : 900, scenario,
          content: rect(reference.content),
          providers: reference.providers.map(provider => ({ name: provider.name, frame: rect(provider.frame) })),
          controls: reference.controls.filter(control => control.frame.width > 0).map(control => ({
            tag: control.tag, label: control.label?.trim() || control.placeholder || (control.tag === 'INPUT' ? '语速' : '标准 DeepL'),
            enabled: !control.disabled, frame: rect(control.frame),
            ...(control.label === '保存策略' ? { appearance: { foreground: color(control.frame.color), background: color(control.frame.background) } } : {}),
          })),
        });
      }
    }
  }
}
const fixture = '{\n  "source": "Actual retained React SettingsWindow Services rendered by WKWebView; rectangles are [x,y,width,height] in logical points.",\n  "scenes": [\n'
  + scenes.map(scene => `    ${JSON.stringify(scene)}`).join(',\n') + '\n  ]\n}\n';
writeFileSync(join(root, 'crates/ui/tests/fixtures/react-services-layout.json'), fixture);
console.log(`Recorded ${scenes.length} actual React Services scenes.`);
