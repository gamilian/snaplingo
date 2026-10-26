import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';

const directory = fileURLToPath(new URL('.', import.meta.url));
await build({
  absWorkingDir: directory,
  entryPoints: ['app.jsx'],
  outfile: 'preview.js',
  bundle: true,
  minify: true,
  format: 'esm',
  target: ['safari15', 'chrome105'],
  define: { 'process.env.NODE_ENV': '"production"' },
  logLevel: 'info',
});
