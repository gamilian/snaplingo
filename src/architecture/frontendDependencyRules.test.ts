import ts from 'typescript';
import { describe, expect, test } from 'vitest';

interface SourceFile {
  path: string;
  source: string;
}

interface TauriEventUse {
  path: string;
  eventName: string;
}

interface ModuleImport {
  path: string;
  specifier: string;
}

const TAURI_EVENT_LISTENER_NAMES = new Set([
  'listenTauriEvent',
  'listenForEvent',
  'listenForHotkey',
]);

const LEGACY_TAURI_ROOT = ['src', 'tauri'].join('/');

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

function allSourceFiles(): SourceFile[] {
  const modules = import.meta.glob('../**/*.{ts,tsx}', {
    eager: true,
    import: 'default',
    query: '?raw',
  }) as Record<string, string>;

  return Object.entries(modules).map(([path, source]) => ({
    path: `src/${path.replace(/^\.\.\//, '')}`,
    source,
  }));
}

function viewSourceFilesIncludingTests(): SourceFile[] {
  const modules = import.meta.glob('../views/**/*.{ts,tsx}', {
    eager: true,
    import: 'default',
    query: '?raw',
  }) as Record<string, string>;

  return Object.entries(modules).map(([path, source]) => ({
    path: `src/${path.replace(/^\.\.\//, '')}`,
    source,
  }));
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

function moduleImports(files: SourceFile[]): ModuleImport[] {
  return files.flatMap((file) => {
    const sourceFile = parseSourceFile(file);
    const imports: ModuleImport[] = [];

    function dynamicImportSpecifier(node: ts.Expression) {
      if (
        ts.isStringLiteral(node) ||
        ts.isNoSubstitutionTemplateLiteral(node)
      ) {
        return node.text;
      }

      if (ts.isTemplateExpression(node)) {
        return node.templateSpans.reduce(
          (specifier, span) => `${specifier}\${...}${span.literal.text}`,
          node.head.text,
        );
      }

      return null;
    }

    function visit(node: ts.Node) {
      if (
        (ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) &&
        node.moduleSpecifier &&
        ts.isStringLiteral(node.moduleSpecifier)
      ) {
        imports.push({ path: file.path, specifier: node.moduleSpecifier.text });
      } else if (
        ts.isCallExpression(node) &&
        node.expression.kind === ts.SyntaxKind.ImportKeyword
      ) {
        const specifier = node.arguments[0];
        if (specifier) {
          const text = dynamicImportSpecifier(specifier);
          if (text !== null) {
            imports.push({ path: file.path, specifier: text });
          }
        }
      }

      ts.forEachChild(node, visit);
    }

    visit(sourceFile);

    return imports;
  });
}

function forbiddenPlatformImports(files: SourceFile[]) {
  return moduleImports(files)
    .filter(({ path, specifier }) => {
      if (path.startsWith('src/platform/tauri/')) return false;
      if (specifier.startsWith('@tauri-apps/')) return true;
      if (/(?:^|\/)platform(?:\/|$)/.test(specifier)) {
        return path !== 'src/App.tsx';
      }
      return /(?:^|\/)tauri(?:\/|$)/.test(specifier);
    })
    .map(({ path, specifier }) => `${path} -> ${specifier}`)
    .sort();
}

function forbiddenApplicationViewImports(files: SourceFile[]) {
  return moduleImports(files)
    .filter(
      ({ path, specifier }) =>
        path.startsWith('src/application/') &&
        /(?:^|\/)views(?:\/|$)/.test(specifier),
    )
    .map(({ path, specifier }) => `${path} -> ${specifier}`)
    .sort();
}

function tauriEventUses(files: SourceFile[]): TauriEventUse[] {
  return files.flatMap((file) => {
    const sourceFile = parseSourceFile(file);
    const listenerNames = new Set(TAURI_EVENT_LISTENER_NAMES);
    const listenerNamespaces = new Set<string>();
    const localStrings = new Map<string, string>();
    const eventNames = new Set<string>();

    function visitDeclarations(node: ts.Node) {
      if (
        ts.isImportDeclaration(node) &&
        ts.isStringLiteral(node.moduleSpecifier) &&
        node.moduleSpecifier.text.endsWith('/tauri/events') &&
        node.importClause?.namedBindings
      ) {
        if (ts.isNamespaceImport(node.importClause.namedBindings)) {
          listenerNamespaces.add(node.importClause.namedBindings.name.text);
        } else {
          for (const element of node.importClause.namedBindings.elements) {
            const importedName = element.propertyName?.text ?? element.name.text;
            if (TAURI_EVENT_LISTENER_NAMES.has(importedName)) {
              listenerNames.add(element.name.text);
            }
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

    function isRecognizedListenerCall(expression: ts.Expression) {
      if (ts.isIdentifier(expression)) {
        return listenerNames.has(expression.text);
      }

      return (
        ts.isPropertyAccessExpression(expression) &&
        ts.isIdentifier(expression.expression) &&
        listenerNamespaces.has(expression.expression.text) &&
        TAURI_EVENT_LISTENER_NAMES.has(expression.name.text)
      );
    }

    function unwrapEventArgument(expression: ts.Expression): ts.Expression {
      let unwrapped = expression;
      while (
        ts.isAsExpression(unwrapped) ||
        ts.isTypeAssertionExpression(unwrapped) ||
        ts.isParenthesizedExpression(unwrapped) ||
        ts.isNonNullExpression(unwrapped)
      ) {
        unwrapped = unwrapped.expression;
      }
      return unwrapped;
    }

    function visitCalls(node: ts.Node) {
      if (
        ts.isCallExpression(node) &&
        isRecognizedListenerCall(node.expression)
      ) {
        const eventArgument = node.arguments[0]
          ? unwrapEventArgument(node.arguments[0])
          : undefined;
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

    // Semantic forwarding or barrel renaming without a recognizable imported
    // listener symbol is outside this static rule. A wrapper that calls a
    // recognized listener with an unresolved identifier is still rejected.

    visitDeclarations(sourceFile);
    visitCalls(sourceFile);

    return Array.from(eventNames, (eventName) => ({
      path: file.path,
      eventName,
    }));
  });
}

function unexpectedTauriEventUses(files: SourceFile[]) {
  return tauriEventUses(files)
    .filter(({ path, eventName }) => {
      if (path.startsWith('src/platform/tauri/')) return false;
      return eventName.length > 0;
    })
    .map(({ path, eventName }) => `${path} -> ${eventName}`)
    .sort();
}

describe('frontend dependency rules', () => {
  test('production Views live under canonical roots and consume Application seams only', () => {
    const productionFiles = productionSourceFiles();
    const paths = productionFiles.map(({ path }) => path);
    const canonicalRoots = [
      'src/views/CaptureWorkspace/',
      'src/views/ResultWindow/',
      'src/views/PinnedImageWindow/',
      'src/views/SettingsWindow/',
    ];

    for (const root of canonicalRoots) {
      expect(paths.some((path) => path.startsWith(root))).toBe(true);
    }

    expect(
      paths.filter((path) =>
        [
          'src/components/ScreenshotSession/',
          'src/components/ResultWindow/',
          'src/components/PinnedImageWindow/',
          'src/components/SettingsWindow/',
        ].some((root) => path.startsWith(root)),
      ),
    ).toEqual([]);

    expect(
      moduleImports(
        productionFiles.filter(({ path }) => path.startsWith('src/views/')),
      )
        .filter(({ specifier }) =>
          specifier.includes('/platform/') ||
          specifier.includes('/tauri/') ||
          specifier.startsWith('@tauri-apps/'),
        )
        .map(({ path, specifier }) => `${path} -> ${specifier}`),
    ).toEqual([]);
  });

  test('View tests also stay independent of legacy and Platform modules', () => {
    expect(
      moduleImports(viewSourceFilesIncludingTests())
        .filter(({ specifier }) =>
          specifier.startsWith('@tauri-apps/') ||
          /(?:^|\/)platform(?:\/|$)/.test(specifier) ||
          /(?:^|\/)tauri(?:\/|$)/.test(specifier),
        )
        .map(({ path, specifier }) => `${path} -> ${specifier}`),
    ).toEqual([]);
  });

  test('App composes Platform adapters into Application runtimes', () => {
    const appFile = productionSourceFiles().find(({ path }) => path === 'src/App.tsx');
    expect(appFile).toBeDefined();

    const imports = moduleImports([appFile!]).map(({ specifier }) => specifier);

    expect(imports).toEqual(
      expect.arrayContaining([
        './application/capture-workspace/platformRuntime',
        './application/result-window/platformRuntime',
        './application/pinned-image/platformRuntime',
        './application/settings/runtime',
        './views/CaptureWorkspace',
        './views/ResultWindow',
        './views/PinnedImageWindow',
        './views/SettingsWindow',
      ]),
    );
    expect(imports.some((specifier) => specifier.startsWith('./platform/tauri/'))).toBe(true);
    expect(imports.some((specifier) => specifier.startsWith('./tauri/'))).toBe(false);
    expect(imports.some((specifier) => specifier.startsWith('@tauri-apps/'))).toBe(false);
  });

  test('the legacy frontend Tauri seam is deleted', () => {
    expect(
      allSourceFiles()
        .map(({ path }) => path)
        .filter((path) => path.startsWith(`${LEGACY_TAURI_ROOT}/`)),
    ).toEqual([]);
  });

  test('only the Tauri platform boundary imports Platform or Tauri modules', () => {
    expect(forbiddenPlatformImports(allSourceFiles())).toEqual([]);
  });

  test('Application modules do not depend on Views', () => {
    expect(forbiddenApplicationViewImports(productionSourceFiles())).toEqual([]);
  });

  test('raw Tauri event strings stay behind the Tauri platform boundary', () => {
    expect(unexpectedTauriEventUses(allSourceFiles())).toEqual([]);
  });

  test('rejects a new @tauri-apps import from a View', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: "import { invoke } from '@tauri-apps/api/core';",
    };

    expect(
      forbiddenPlatformImports([syntheticView]),
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
      unexpectedTauriEventUses([syntheticView]),
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
      unexpectedTauriEventUses([syntheticView]),
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
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual(['src/views/ExampleView.tsx -> new-event']);
  });

  test('rejects no-substitution template event literals', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: 'listenTauriEvent(`new-event`, handler);',
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
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

    expect(forbiddenPlatformImports([syntheticView])).toEqual([]);
    expect(unexpectedTauriEventUses([syntheticView])).toEqual([]);
  });

  test('rejects dynamic @tauri-apps imports outside the platform boundary', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: "const tauri = await import('@tauri-apps/api/core');",
    };

    expect(
      forbiddenPlatformImports([syntheticView]),
    ).toEqual(['src/views/ExampleView.tsx -> @tauri-apps/api/core']);
  });

  test('rejects interpolated dynamic imports across frontend boundaries', () => {
    const syntheticFiles: SourceFile[] = [
      {
        path: 'src/views/TauriView.tsx',
        source: "const tauri = await import(`@tauri-apps/api/${api}`);",
      },
      {
        path: 'src/views/PlatformView.tsx',
        source: "const adapter = await import(`../platform/${adapter}`);",
      },
      {
        path: 'src/application/legacy.ts',
        source: "const legacy = await import(`../tauri/${adapter}`);",
      },
    ];

    expect(forbiddenPlatformImports(syntheticFiles)).toEqual([
      'src/application/legacy.ts -> ../tauri/${...}',
      'src/views/PlatformView.tsx -> ../platform/${...}',
      'src/views/TauriView.tsx -> @tauri-apps/api/${...}',
    ]);
  });

  test('rejects @tauri-apps re-exports outside the platform boundary', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: "export { invoke } from '@tauri-apps/api/core';",
    };

    expect(
      forbiddenPlatformImports([syntheticView]),
    ).toEqual(['src/views/ExampleView.tsx -> @tauri-apps/api/core']);
  });

  test('rejects events passed through an imported listener namespace', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.tsx',
      source: `
        import * as events from '../tauri/events';
        events.listenTauriEvent('new-event', handler);
      `,
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual(['src/views/ExampleView.tsx -> new-event']);
  });

  test('unwraps transparent TypeScript expressions around event arguments', () => {
    const syntheticView: SourceFile = {
      path: 'src/views/ExampleView.ts',
      source: `
        listenTauriEvent('as-event' as EventName, handler);
        listenTauriEvent(<EventName>'asserted-event', handler);
        listenTauriEvent(('parenthesized-event'), handler);
        listenTauriEvent('nonnull-event'!, handler);
      `,
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual([
      'src/views/ExampleView.ts -> as-event',
      'src/views/ExampleView.ts -> asserted-event',
      'src/views/ExampleView.ts -> nonnull-event',
      'src/views/ExampleView.ts -> parenthesized-event',
    ]);
  });
});
