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

const LEGACY_TAURI_ROOT = ['reference', 'web', 'tauri'].join('/');
const VIEW_ROOT_NAMES = new Set([
  'CaptureWorkspace',
  'PinnedImageWindow',
  'ResultWindow',
  'SettingsWindow',
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
      path: `reference/web/${path.replace(/^\.\.\//, '')}`,
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
    path: `reference/web/${path.replace(/^\.\.\//, '')}`,
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
    path: `reference/web/${path.replace(/^\.\.\//, '')}`,
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
    .filter(({ specifier }) => {
      if (specifier.startsWith('@tauri-apps/')) return true;
      if (/(?:^|\/)platform(?:\/|$)/.test(specifier)) {
        return true;
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
        path.startsWith('reference/web/application/') &&
        /(?:^|\/)views(?:\/|$)/.test(specifier),
    )
    .map(({ path, specifier }) => `${path} -> ${specifier}`)
    .sort();
}

function crossViewRootImports(files: SourceFile[]) {
  return moduleImports(files)
    .filter(({ path, specifier }) => {
      const sourceRoot = path.match(/^reference\/web\/views\/([^/]+)\//)?.[1];
      if (!sourceRoot) return false;

      const targetRoot = specifier
        .split('/')
        .find((segment) => VIEW_ROOT_NAMES.has(segment));
      return targetRoot !== undefined && targetRoot !== sourceRoot;
    })
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
    .filter(({ eventName }) => {
      return eventName.length > 0;
    })
    .map(({ path, eventName }) => `${path} -> ${eventName}`)
    .sort();
}

describe('frontend dependency rules', () => {
  test('Capture workflow implementation stays in Application and uses DOM adapters', () => {
    const files = productionSourceFiles();
    const captureFiles = files.filter(({ path }) =>
      path.startsWith('reference/web/application/capture-workspace/'),
    );
    expect(captureFiles.some(({ path }) => path.endsWith('/runtime.ts'))).toBe(true);
    expect(moduleImports(captureFiles).filter(({ specifier }) =>
      specifier.includes('/views/') || specifier.includes('/components/') ||
      specifier === 'react' || specifier.startsWith('react-dom'),
    )).toEqual([]);
    expect(captureFiles.filter(({ source }) =>
      /\b(?:document|HTMLImageElement|HTMLCanvasElement|KeyboardEvent)\b/.test(source),
    ).map(({ path }) => path)).toEqual([]);
    expect(files.some(({ path }) =>
      path === 'reference/web/views/CaptureWorkspace/captureWorkspaceRuntime.ts' ||
      path === 'reference/web/application/capture-workspace/platformRuntime.ts',
    )).toBe(false);
    const runtime = captureFiles.find(({ path }) => path.endsWith('/runtime.ts'))!;
    expect(runtime.source).toContain('platform.print.printImage(imageBase64)');
  });

  test('all Application modules keep browser mechanics behind DOM adapters', () => {
    const forbidden = new Set(['HTMLImageElement', 'HTMLCanvasElement', 'KeyboardEvent', 'MouseEvent', 'PointerEvent', 'WheelEvent', 'document']);
    const violations: string[] = [];
    for (const file of productionSourceFiles().filter(({ path }) => path.startsWith('reference/web/application/'))) {
      const source = parseSourceFile(file);
      function visit(node: ts.Node) {
        if (ts.isIdentifier(node) && forbidden.has(node.text)) {
          violations.push(`${file.path}: ${node.text}`);
        }
        ts.forEachChild(node, visit);
      }
      visit(source);
    }
    expect([...new Set(violations)]).toEqual([]);
  });

  test('Domain data does not import workflows or adapters', () => {
    const violations = moduleImports(productionSourceFiles())
      .filter(({ path, specifier }) => path.startsWith('reference/web/domain/') && /(?:^|\/)(?:application|platform|views|stores)(?:\/|$)|^@tauri-apps\//.test(specifier));
    expect(violations).toEqual([]);
  });

  test('production Views live under canonical roots and consume Application seams only', () => {
    const productionFiles = productionSourceFiles();
    const paths = productionFiles.map(({ path }) => path);
    const canonicalRoots = [
      'reference/web/views/CaptureWorkspace/',
      'reference/web/views/ResultWindow/',
      'reference/web/views/PinnedImageWindow/',
      'reference/web/views/SettingsWindow/',
    ];

    for (const root of canonicalRoots) {
      expect(paths.some((path) => path.startsWith(root))).toBe(true);
    }

    expect(
      paths.filter((path) =>
        [
          'reference/web/components/ScreenshotSession/',
          'reference/web/components/ResultWindow/',
          'reference/web/components/PinnedImageWindow/',
          'reference/web/components/SettingsWindow/',
        ].some((root) => path.startsWith(root)),
      ),
    ).toEqual([]);

    expect(
      moduleImports(
        productionFiles.filter(({ path }) => path.startsWith('reference/web/views/')),
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

  test('View roots share application knowledge without importing each other', () => {
    expect(crossViewRootImports(viewSourceFilesIncludingTests())).toEqual([]);
  });

  test('the browser entry loads only isolated visual references', () => {
    const appFile = productionSourceFiles().find(({ path }) => path === 'reference/web/App.tsx');
    expect(appFile).toBeDefined();
    const imports = moduleImports([appFile!]).map(({ specifier }) => specifier);
    expect(imports).toEqual(['react']);
    expect(appFile!.source).toContain('/fixtures/');
    expect(appFile!.source).toContain('referenceUrl(window.location.search)');
    expect(allSourceFiles().some(({ path }) => path === 'reference/web/CaptureApp.tsx')).toBe(false);
  });

  test('Settings configuration lifecycle stays behind the Application seam', () => {
    const productionFiles = productionSourceFiles();
    const storePaths = new Set([
      'reference/web/stores/settingsConfigStore.ts',
      'reference/web/stores/hotkeyConfigStore.ts',
      'reference/web/stores/providerStore.ts',
    ]);
    const storeFiles = productionFiles.filter(({ path }) => storePaths.has(path));
    const imports = moduleImports(storeFiles);

    expect(storeFiles).toHaveLength(storePaths.size);
    expect(
      storeFiles.map(({ path }) => path).filter((path) =>
        imports.some(
          (entry) =>
            entry.path === path &&
            entry.specifier === '../application/settings/configuration',
        ),
      ),
    ).toHaveLength(storePaths.size);
    expect(
      imports
        .filter(({ specifier }) => specifier === '../application/settings/runtime')
        .map(({ path }) => path),
    ).toEqual([]);
  });

  test('Result Window projects application-owned state and scheduling', () => {
    const productionFiles = productionSourceFiles();
    const viewFile = productionFiles.find(
      ({ path }) => path === 'reference/web/views/ResultWindow/ResultWindow.tsx',
    );

    expect(viewFile).toBeDefined();
    expect(viewFile!.source).not.toContain('autoTranslateRequestId');
    expect(viewFile!.source).not.toContain('incrementalTranslation');
    const storeFile = productionFiles.find(({ path }) => path === 'reference/web/stores/resultWindowStore.ts');
    expect(storeFile!.source).toContain('runtime.subscribe(');
    expect(storeFile!.source).not.toContain('startTranslationSession');
    expect(storeFile!.source).not.toContain('completeProviderTranslation');
    expect(viewFile!.source).toContain('useResultWindowProjection()');
    expect(viewFile!.source).not.toContain('\n    setSourceText,');
    expect(viewFile!.source).not.toContain('\n    setSourceLang,');
    expect(viewFile!.source).not.toContain('\n    setTargetLang,');
    expect(viewFile!.source).not.toContain('../../stores/providerStore');
    expect(viewFile!.source).not.toContain('../../stores/settingsConfigStore');
    expect(
      allSourceFiles().some(({ path }) => path === 'reference/web/stores/appStore.ts'),
    ).toBe(false);
  });

  test('the legacy frontend Tauri seam is deleted', () => {
    expect(
      allSourceFiles()
        .map(({ path }) => path)
        .filter((path) => path.startsWith(`${LEGACY_TAURI_ROOT}/`) || path.startsWith('reference/web/platform/tauri/')),
    ).toEqual([]);
  });

  test('reference frontend does not import Platform or Tauri modules', () => {
    expect(forbiddenPlatformImports(allSourceFiles())).toEqual([]);
  });

  test('Application modules do not depend on Views', () => {
    expect(forbiddenApplicationViewImports(productionSourceFiles())).toEqual([]);
  });

  test('the shallow Pinned Image platform runtime is deleted', () => {
    expect(
      allSourceFiles()
        .map(({ path }) => path)
        .filter((path) =>
          path.startsWith('reference/web/application/pinned-image/platformRuntime.'),
        ),
    ).toEqual([]);
  });

  test('reference frontend does not subscribe to desktop Tauri events', () => {
    expect(unexpectedTauriEventUses(allSourceFiles())).toEqual([]);
  });

  test('rejects a new @tauri-apps import from a View', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
      source: "import { invoke } from '@tauri-apps/api/core';",
    };

    expect(
      forbiddenPlatformImports([syntheticView]),
    ).toEqual([
      'reference/web/views/ExampleView.tsx -> @tauri-apps/api/core',
    ]);
  });

  test('rejects local event-name identifiers passed to Tauri listeners', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
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
      'reference/web/views/ExampleView.tsx -> new-event',
    ]);
  });

  test('rejects unresolved event-name identifiers passed to Tauri listeners', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
      source: `
        import { CAPTURE_EVENT } from './events';
        listenTauriEvent(CAPTURE_EVENT, handler);
      `,
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual(['reference/web/views/ExampleView.tsx -> CAPTURE_EVENT']);
  });

  test('rejects events passed through an imported listener alias', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
      source: `
        import { listenTauriEvent as subscribe } from '../tauri/events';
        subscribe('new-event', handler);
      `,
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual(['reference/web/views/ExampleView.tsx -> new-event']);
  });

  test('rejects no-substitution template event literals', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
      source: 'listenTauriEvent(`new-event`, handler);',
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual(['reference/web/views/ExampleView.tsx -> new-event']);
  });

  test('ignores imports and listeners inside comments', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
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
      path: 'reference/web/views/ExampleView.tsx',
      source: "const tauri = await import('@tauri-apps/api/core');",
    };

    expect(
      forbiddenPlatformImports([syntheticView]),
    ).toEqual(['reference/web/views/ExampleView.tsx -> @tauri-apps/api/core']);
  });

  test('rejects interpolated dynamic imports across frontend boundaries', () => {
    const syntheticFiles: SourceFile[] = [
      {
        path: 'reference/web/views/TauriView.tsx',
        source: "const tauri = await import(`@tauri-apps/api/${api}`);",
      },
      {
        path: 'reference/web/views/PlatformView.tsx',
        source: "const adapter = await import(`../platform/${adapter}`);",
      },
      {
        path: 'reference/web/application/legacy.ts',
        source: "const legacy = await import(`../tauri/${adapter}`);",
      },
    ];

    expect(forbiddenPlatformImports(syntheticFiles)).toEqual([
      'reference/web/application/legacy.ts -> ../tauri/${...}',
      'reference/web/views/PlatformView.tsx -> ../platform/${...}',
      'reference/web/views/TauriView.tsx -> @tauri-apps/api/${...}',
    ]);
  });

  test('rejects @tauri-apps re-exports outside the platform boundary', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
      source: "export { invoke } from '@tauri-apps/api/core';",
    };

    expect(
      forbiddenPlatformImports([syntheticView]),
    ).toEqual(['reference/web/views/ExampleView.tsx -> @tauri-apps/api/core']);
  });

  test('rejects events passed through an imported listener namespace', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.tsx',
      source: `
        import * as events from '../tauri/events';
        events.listenTauriEvent('new-event', handler);
      `,
    };

    expect(
      unexpectedTauriEventUses([syntheticView]),
    ).toEqual(['reference/web/views/ExampleView.tsx -> new-event']);
  });

  test('unwraps transparent TypeScript expressions around event arguments', () => {
    const syntheticView: SourceFile = {
      path: 'reference/web/views/ExampleView.ts',
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
      'reference/web/views/ExampleView.ts -> as-event',
      'reference/web/views/ExampleView.ts -> asserted-event',
      'reference/web/views/ExampleView.ts -> nonnull-event',
      'reference/web/views/ExampleView.ts -> parenthesized-event',
    ]);
  });
});
