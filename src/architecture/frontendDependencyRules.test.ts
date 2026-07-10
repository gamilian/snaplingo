import ts from 'typescript';
import { describe, expect, test } from 'vitest';

interface SourceFile {
  path: string;
  source: string;
}

interface TauriImport {
  path: string;
  specifier: string;
}

interface TauriEventUse {
  path: string;
  eventName: string;
}

const LEGACY_TAURI_IMPORTS = new Set([
  'src/tauri/captureSession.ts -> @tauri-apps/api/core',
  'src/tauri/events.ts -> @tauri-apps/api/event',
  'src/tauri/history.ts -> @tauri-apps/api/core',
  'src/tauri/hotkeys.ts -> @tauri-apps/api/core',
  'src/tauri/ocr.ts -> @tauri-apps/api/core',
  'src/tauri/ocr.ts -> @tauri-apps/plugin-dialog',
  'src/tauri/pinnedImage.ts -> @tauri-apps/api/core',
  'src/tauri/providers.ts -> @tauri-apps/api/core',
  'src/tauri/settings.ts -> @tauri-apps/api/core',
  'src/tauri/translation.ts -> @tauri-apps/api/core',
  'src/tauri/window.ts -> @tauri-apps/api/webviewWindow',
  'src/tauri/window.ts -> @tauri-apps/api/window',
]);

const LEGACY_TAURI_EVENT_USES = new Set([
  'src/App.tsx -> capture-result-payload-ready',
  'src/components/ScreenshotSession/captureCancelRequest.ts -> capture-cancel-requested',
  'src/components/ScreenshotSession/captureCancelRequest.ts -> capture-copy-requested',
  'src/components/ScreenshotSession/captureCancelRequest.ts -> eventName',
  'src/components/ScreenshotSession/captureHostRuntime.ts -> eventName',
  'src/components/ScreenshotSession/captureHostRuntime.ts -> hotkey-triggered',
]);

const TAURI_EVENT_LISTENER_NAMES = new Set([
  'listenTauriEvent',
  'listenForEvent',
  'listenForHotkey',
]);

function isProductionTypeScript(path: string) {
  return (
    /\.tsx?$/.test(path) &&
    !/\.(?:test|spec)\.tsx?$/.test(path) &&
    !path.includes('/__tests__/') &&
    !path.includes('.PROTOTYPE.') &&
    !path.includes('/prototypes/') &&
    !path.includes('/generated/') &&
    !path.includes('/node_modules/') &&
    !path.includes('/dist/') &&
    !path.includes('/designs/')
  );
}

function productionSourceFiles(): SourceFile[] {
  const modules = import.meta.glob('../**/*.{ts,tsx}', {
    eager: true,
    import: 'default',
    query: '?raw',
  }) as Record<string, string>;

  return Object.entries(modules)
    .map(([path, source]) => ({
      path: `src/${path.replace(/^\.\.\//, '')}`,
      source,
    }))
    .filter(({ path }) => isProductionTypeScript(path));
}

function parseSourceFile({ path, source }: SourceFile) {
  return ts.createSourceFile(
    path,
    source,
    ts.ScriptTarget.Latest,
    true,
    path.endsWith('.tsx') ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
  );
}

function tauriImports(files: SourceFile[]): TauriImport[] {
  return files.flatMap((file) => {
    const sourceFile = parseSourceFile(file);
    const imports: TauriImport[] = [];

    function recordModuleReference(specifier: string) {
      if (specifier.startsWith('@tauri-apps/')) {
        imports.push({ path: file.path, specifier });
      }
    }

    function visit(node: ts.Node) {
      if (
        (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) &&
        node.moduleSpecifier &&
        ts.isStringLiteral(node.moduleSpecifier)
      ) {
        recordModuleReference(node.moduleSpecifier.text);
      } else if (
        ts.isCallExpression(node) &&
        node.expression.kind === ts.SyntaxKind.ImportKeyword
      ) {
        const specifier = node.arguments[0];
        if (
          specifier &&
          (ts.isStringLiteral(specifier) ||
            ts.isNoSubstitutionTemplateLiteral(specifier))
        ) {
          recordModuleReference(specifier.text);
        }
      }

      ts.forEachChild(node, visit);
    }

    visit(sourceFile);
    return imports;
  });
}

function tauriImportInventoryViolations(
  files: SourceFile[],
  legacyAllowlist: ReadonlySet<string>,
) {
  const actualLegacyImports = new Set(
    tauriImports(files)
      .filter(({ path }) => !path.startsWith('src/platform/tauri/'))
      .map(({ path, specifier }) => `${path} -> ${specifier}`),
  );

  return [
    ...Array.from(actualLegacyImports).filter(
      (entry) => !legacyAllowlist.has(entry),
    ),
    ...Array.from(legacyAllowlist).filter(
      (entry) => !actualLegacyImports.has(entry),
    ),
  ].sort();
}

function tauriEventUses(files: SourceFile[]): TauriEventUse[] {
  return files.flatMap((file) => {
    const sourceFile = parseSourceFile(file);
    const listenerNames = new Set(TAURI_EVENT_LISTENER_NAMES);
    const localStrings = new Map<string, string>();
    const eventNames = new Set<string>();

    function visitDeclarations(node: ts.Node) {
      if (
        ts.isImportDeclaration(node) &&
        ts.isStringLiteral(node.moduleSpecifier) &&
        node.moduleSpecifier.text.endsWith('/tauri/events') &&
        node.importClause?.namedBindings &&
        ts.isNamedImports(node.importClause.namedBindings)
      ) {
        for (const element of node.importClause.namedBindings.elements) {
          const importedName = element.propertyName?.text ?? element.name.text;
          if (TAURI_EVENT_LISTENER_NAMES.has(importedName)) {
            listenerNames.add(element.name.text);
          }
        }
      }

      if (
        ts.isVariableDeclaration(node) &&
        ts.isIdentifier(node.name) &&
        ts.isVariableDeclarationList(node.parent) &&
        (node.parent.flags & ts.NodeFlags.Const) !== 0 &&
        node.initializer &&
        (ts.isStringLiteral(node.initializer) ||
          ts.isNoSubstitutionTemplateLiteral(node.initializer))
      ) {
        localStrings.set(node.name.text, node.initializer.text);
      }

      ts.forEachChild(node, visitDeclarations);
    }

    function visitCalls(node: ts.Node) {
      if (
        ts.isCallExpression(node) &&
        ts.isIdentifier(node.expression) &&
        listenerNames.has(node.expression.text)
      ) {
        const eventArgument = node.arguments[0];
        if (
          eventArgument &&
          (ts.isStringLiteral(eventArgument) ||
            ts.isNoSubstitutionTemplateLiteral(eventArgument))
        ) {
          eventNames.add(eventArgument.text);
        } else if (eventArgument && ts.isIdentifier(eventArgument)) {
          eventNames.add(
            localStrings.get(eventArgument.text) ?? eventArgument.text,
          );
        }
      }

      ts.forEachChild(node, visitCalls);
    }

    visitDeclarations(sourceFile);
    visitCalls(sourceFile);

    return Array.from(eventNames, (eventName) => ({
      path: file.path,
      eventName,
    }));
  });
}

function unexpectedTauriEventUses(
  files: SourceFile[],
  legacyAllowlist: ReadonlySet<string>,
) {
  return tauriEventUses(files)
    .filter(({ path, eventName }) => {
      if (path.startsWith('src/platform/tauri/')) return false;
      return !legacyAllowlist.has(`${path} -> ${eventName}`);
    })
    .map(({ path, eventName }) => `${path} -> ${eventName}`)
    .sort();
}

describe('frontend dependency rules', () => {
  test('@tauri-apps imports stay behind the Tauri platform boundary', () => {
    const productionFiles = productionSourceFiles();
    const legacyInventory = tauriImports(productionFiles)
      .filter(({ path }) => !path.startsWith('src/platform/tauri/'))
      .map(({ path, specifier }) => `${path} -> ${specifier}`)
      .sort();

    expect(legacyInventory).toEqual(Array.from(LEGACY_TAURI_IMPORTS).sort());
    expect(
      tauriImportInventoryViolations(productionFiles, LEGACY_TAURI_IMPORTS),
    ).toEqual([]);
  });

  test('raw Tauri event strings stay behind the Tauri platform boundary', () => {
    const productionFiles = productionSourceFiles();
    const legacyInventory = tauriEventUses(productionFiles)
      .filter(({ path }) => !path.startsWith('src/platform/tauri/'))
      .map(({ path, eventName }) => `${path} -> ${eventName}`)
      .sort();

    expect(legacyInventory).toEqual(
      Array.from(LEGACY_TAURI_EVENT_USES).sort(),
    );
    expect(
      unexpectedTauriEventUses(
        productionFiles,
        LEGACY_TAURI_EVENT_USES,
      ),
    ).toEqual([]);
  });

  test('rejects a new @tauri-apps import from a View', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: "import { invoke } from '@tauri-apps/api/core';",
    };

    expect(
      tauriImportInventoryViolations([syntheticView], new Set()),
    ).toEqual([
      'src/views/ExampleView.tsx -> @tauri-apps/api/core',
    ]);
  });

  test('rejects local event-name identifiers passed to Tauri listeners', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: `
        const eventName = 'new-event';
        const typedEventName = 'new-typed-event';
        const domEventName = 'click';
        const CLICK_EVENT = 'click';
        listenTauriEvent(eventName, handler);
        listenForTypedEvent(typedEventName, handler);
        window.addEventListener(domEventName, handler);
        window.addEventListener(CLICK_EVENT, handler);
        subscribeNewsletter('weekly');
      `,
    };

    expect(
      unexpectedTauriEventUses(
        [syntheticView],
        LEGACY_TAURI_EVENT_USES,
      ),
    ).toEqual([
      'src/views/ExampleView.tsx -> new-event',
    ]);
  });

  test('rejects unresolved event-name identifiers passed to Tauri listeners', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: `
        import { CAPTURE_EVENT } from './events';
        listenTauriEvent(CAPTURE_EVENT, handler);
      `,
    };

    expect(
      unexpectedTauriEventUses(
        [syntheticView],
        LEGACY_TAURI_EVENT_USES,
      ),
    ).toEqual(['src/views/ExampleView.tsx -> CAPTURE_EVENT']);
  });

  test('rejects events passed through an imported listener alias', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: `
        import { listenTauriEvent as subscribe } from '../tauri/events';
        subscribe('new-event', handler);
      `,
    };

    expect(
      unexpectedTauriEventUses([syntheticView], LEGACY_TAURI_EVENT_USES),
    ).toEqual(['src/views/ExampleView.tsx -> new-event']);
  });

  test('rejects no-substitution template event literals', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: 'listenTauriEvent(`new-event`, handler);',
    };

    expect(
      unexpectedTauriEventUses([syntheticView], LEGACY_TAURI_EVENT_USES),
    ).toEqual(['src/views/ExampleView.tsx -> new-event']);
  });

  test('ignores imports and listeners inside comments', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: `
        // import { invoke } from '@tauri-apps/api/core';
        /* listenTauriEvent('commented-event', handler); */
      `,
    };

    expect(tauriImportInventoryViolations([syntheticView], new Set())).toEqual(
      [],
    );
    expect(unexpectedTauriEventUses([syntheticView], new Set())).toEqual([]);
  });

  test('rejects stale legacy Tauri import allowlist entries', () => {
    const staleAllowlist = new Set([
      'src/tauri/removed.ts -> @tauri-apps/api/core',
    ]);

    expect(tauriImportInventoryViolations([], staleAllowlist)).toEqual([
      'src/tauri/removed.ts -> @tauri-apps/api/core',
    ]);
  });

  test('rejects dynamic @tauri-apps imports outside the platform boundary', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: "const tauri = await import('@tauri-apps/api/core');",
    };

    expect(
      tauriImportInventoryViolations([syntheticView], new Set()),
    ).toEqual(['src/views/ExampleView.tsx -> @tauri-apps/api/core']);
  });

  test('rejects @tauri-apps re-exports outside the platform boundary', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: "export { invoke } from '@tauri-apps/api/core';",
    };

    expect(
      tauriImportInventoryViolations([syntheticView], new Set()),
    ).toEqual(['src/views/ExampleView.tsx -> @tauri-apps/api/core']);
  });
});
