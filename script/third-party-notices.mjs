import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, 'src/domain/thirdPartyNotices.json');
const hash = (value) => createHash('sha256').update(value).digest('hex');
const lockfiles = Object.fromEntries(['Cargo.lock', 'package-lock.json', 'script/ocr-assets.json'].map((file) => [file, hash(readFileSync(join(root, file)))]));

if (process.argv.includes('--check')) {
  const inventory = JSON.parse(readFileSync(output, 'utf8'));
  if (JSON.stringify(inventory.lockfiles) !== JSON.stringify(lockfiles)) throw new Error('Third-party notices are stale. Run npm run licenses:generate.');
  console.log('Third-party notices match the dependency locks.');
} else {
  const metadataResult = spawnSync('cargo', ['metadata', '--format-version', '1', '--locked'], { cwd: root, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  if (metadataResult.status !== 0) throw new Error(metadataResult.stderr);
  const metadata = JSON.parse(metadataResult.stdout);
  const texts = {};
  const components = [];
  const notice = (path) => {
    const text = readFileSync(path, 'utf8').replaceAll('\r\n', '\n').trim();
    const id = hash(text); texts[id] = text; return id;
  };
  const noticeFiles = (directory) => readdirSync(directory, { withFileTypes: true }).filter((entry) => entry.isFile() && /^(licen[cs]e|copying|notice)([._-]|$)/i.test(entry.name)).map((entry) => notice(join(directory, entry.name)));
  const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
  const included = new Set();
  const visit = (id) => {
    if (included.has(id)) return;
    included.add(id);
    for (const dependency of nodes.get(id)?.deps ?? []) {
      if (dependency.dep_kinds.some((kind) => kind.kind !== 'dev')) visit(dependency.pkg);
    }
  };
  for (const id of metadata.workspace_default_members) visit(id);
  for (const pkg of metadata.packages.filter((pkg) => included.has(pkg.id) && !metadata.workspace_members.includes(pkg.id))) {
    const notices = noticeFiles(dirname(pkg.manifest_path));
    if (pkg.license_file && existsSync(pkg.license_file)) notices.push(notice(pkg.license_file));
    components.push({ name: pkg.name, version: pkg.version, license: pkg.license ?? 'See upstream notice', source: pkg.repository ?? pkg.homepage ?? `https://crates.io/crates/${pkg.name}/${pkg.version}`, category: 'Rust', notices: [...new Set(notices)] });
  }
  const npmLock = JSON.parse(readFileSync(join(root, 'package-lock.json'), 'utf8'));
  for (const [path, pkg] of Object.entries(npmLock.packages).filter(([path, pkg]) => path.startsWith('node_modules/') && !pkg.dev)) {
    const directory = join(root, path);
    const manifest = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'));
    const repository = typeof manifest.repository === 'string' ? manifest.repository : manifest.repository?.url;
    let source = repository?.replace(/^git\+/, '').replace(/^git:\/\//, 'https://').replace(/\.git$/, '') ?? `https://www.npmjs.com/package/${manifest.name}`;
    if (!source.startsWith('https://') && !source.startsWith('http://')) source = `https://github.com/${source}`;
    components.push({ name: manifest.name, version: pkg.version, license: manifest.license ?? pkg.license ?? 'See upstream notice', source, category: 'JavaScript', notices: noticeFiles(directory) });
  }
  components.push({ name: 'Noto Sans CJK SC', version: 'Bundled font', license: 'OFL-1.1', source: 'https://github.com/notofonts/noto-cjk', category: '资源', notices: [notice(join(root, 'src-tauri/assets/fonts/LICENSE-NotoSans.txt'))] });
  const assets = JSON.parse(readFileSync(join(root, 'script/ocr-assets.json'), 'utf8'));
  for (const asset of assets.models.filter((asset) => asset.file.startsWith('licenses/'))) {
    const path = join(root, 'src-tauri/resources/ocr', asset.file);
    if (existsSync(path)) components.push({ name: asset.file.includes('paddle') ? 'PaddleOCR models' : 'ocr-rs models', version: 'Bundled models', license: 'Apache-2.0', source: asset.url, category: '资源', notices: [notice(path)] });
  }
  for (const [name, file, source, license] of [
    ['ONNX Runtime', 'onnxruntime.txt', 'https://github.com/microsoft/onnxruntime', 'MIT'],
    ['ONNX Runtime third-party notices', 'onnxruntime-third-party.txt', 'https://github.com/microsoft/onnxruntime', 'See notices'],
    ['MNN', 'mnn/LICENSE-MNN.txt', 'https://github.com/alibaba/MNN', 'Apache-2.0'],
  ]) {
    const path = join(root, 'src-tauri/resources/ocr/licenses', file);
    if (existsSync(path)) components.push({ name, version: 'Bundled runtime', license, source, category: '资源', notices: [notice(path)] });
  }
  const mnnThirdParty = join(root, 'src-tauri/resources/ocr/licenses/mnn/licenses/3rd_party');
  if (existsSync(mnnThirdParty)) for (const name of readdirSync(mnnThirdParty)) {
    components.push({ name: `MNN / ${name}`, version: 'Bundled runtime', license: 'See notices', source: 'https://github.com/alibaba/MNN/tree/master/3rd_party', category: '资源', notices: noticeFiles(join(mnnThirdParty, name)) });
  }
  components.sort((a, b) => a.category.localeCompare(b.category) || a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
  writeFileSync(output, JSON.stringify({ lockfiles, components, texts }, null, 2) + '\n');
  console.log(`Recorded ${components.length} third-party components and ${Object.keys(texts).length} notice texts.`);
}
