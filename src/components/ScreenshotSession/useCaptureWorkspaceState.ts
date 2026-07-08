import {
  useCallback,
  useMemo,
  useRef,
  useState,
  type MutableRefObject,
  type SetStateAction,
} from 'react';

import type { LoadedCaptureHostSession } from './captureHostRuntime';
import {
  type CaptureWorkspaceState,
  createInitialCaptureWorkspaceState,
  loadedCaptureHostSessionPatch,
  previewResetPatch,
  resetCaptureInteractionStatePatch,
} from './captureWorkspaceState';
import type { LogicalRect, Point } from './types';

export interface CaptureWorkspaceRefs {
  startPointRef: MutableRefObject<Point | null>;
  cursorPointRef: MutableRefObject<Point | null>;
  draftSelectionRef: MutableRefObject<LogicalRect | null>;
  hoverSelectionRef: MutableRefObject<LogicalRect | null>;
}

export interface UseCaptureWorkspaceStateOptions {
  onRenderingOutputChange?: (isRendering: boolean) => void;
  onHoverSelectionSynced?: (nextHoverSelection: LogicalRect | null) => void;
}

interface CaptureWorkspaceStateActionOptions
  extends UseCaptureWorkspaceStateOptions {
  refs: CaptureWorkspaceRefs;
  applyPatch: (patch: Partial<CaptureWorkspaceState>) => void;
}

export function applyCaptureWorkspaceStatePatch(
  state: CaptureWorkspaceState,
  patch: Partial<CaptureWorkspaceState>,
): CaptureWorkspaceState {
  return {
    ...state,
    ...patch,
  };
}

export function clearCaptureWorkspaceRefs(refs: CaptureWorkspaceRefs) {
  refs.startPointRef.current = null;
  refs.cursorPointRef.current = null;
  refs.draftSelectionRef.current = null;
  refs.hoverSelectionRef.current = null;
}

export function syncCaptureWorkspaceRefsFromPatch(
  refs: CaptureWorkspaceRefs,
  patch: Partial<CaptureWorkspaceState>,
) {
  if ('startPoint' in patch) {
    refs.startPointRef.current = patch.startPoint ?? null;
  }
  if ('cursorPoint' in patch) {
    refs.cursorPointRef.current = patch.cursorPoint ?? null;
  }
  if ('hoverSelection' in patch) {
    refs.hoverSelectionRef.current = patch.hoverSelection ?? null;
  }
}

export function createCaptureWorkspaceStateActions({
  refs,
  applyPatch,
  onRenderingOutputChange,
  onHoverSelectionSynced,
}: CaptureWorkspaceStateActionOptions) {
  return {
    setStartPointWithRef(point: Point | null) {
      refs.startPointRef.current = point;
      applyPatch({ startPoint: point });
    },
    setCursorPointWithRef(point: Point | null) {
      refs.cursorPointRef.current = point;
      applyPatch({ cursorPoint: point });
    },
    setDraftSelectionWithRef(rect: LogicalRect | null) {
      refs.draftSelectionRef.current = rect;
    },
    syncHoverSelection(nextHoverSelection: LogicalRect | null) {
      if (areRectsEqual(refs.hoverSelectionRef.current, nextHoverSelection)) return;

      refs.hoverSelectionRef.current = nextHoverSelection;
      applyPatch({ hoverSelection: nextHoverSelection });
      onHoverSelectionSynced?.(nextHoverSelection);
    },
    setRenderingOutput(nextIsRendering: boolean) {
      applyPatch({ isRenderingOutput: nextIsRendering });
      onRenderingOutputChange?.(nextIsRendering);
    },
  };
}

export function useCaptureWorkspaceState(
  options: UseCaptureWorkspaceStateOptions = {},
) {
  const [state, setState] = useState(createInitialCaptureWorkspaceState);
  const startPointRef = useRef<Point | null>(null);
  const cursorPointRef = useRef<Point | null>(null);
  const draftSelectionRef = useRef<LogicalRect | null>(null);
  const hoverSelectionRef = useRef<LogicalRect | null>(null);
  const refs = useMemo<CaptureWorkspaceRefs>(
    () => ({
      startPointRef,
      cursorPointRef,
      draftSelectionRef,
      hoverSelectionRef,
    }),
    [],
  );

  const applyPatch = useCallback(
    (patch: Partial<CaptureWorkspaceState>) => {
      setState((currentState) =>
        applyCaptureWorkspaceStatePatch(currentState, patch),
      );
      if ('isRenderingOutput' in patch && patch.isRenderingOutput !== undefined) {
        options.onRenderingOutputChange?.(patch.isRenderingOutput);
      }
    },
    [options.onRenderingOutputChange],
  );

  const setWorkspaceField = useCallback(
    <Field extends keyof CaptureWorkspaceState>(
      field: Field,
      nextValue: SetStateAction<CaptureWorkspaceState[Field]>,
    ) => {
      setState((currentState) => {
        const resolvedValue = resolveSetStateAction(
          currentState[field],
          nextValue,
        );

        if (Object.is(currentState[field], resolvedValue)) {
          return currentState;
        }

        return applyCaptureWorkspaceStatePatch(currentState, {
          [field]: resolvedValue,
        } as Partial<CaptureWorkspaceState>);
      });
    },
    [],
  );

  const fieldSetters = useMemo(() => {
    const createFieldSetter =
      <Field extends keyof CaptureWorkspaceState>(field: Field) =>
      (nextValue: SetStateAction<CaptureWorkspaceState[Field]>) => {
        setWorkspaceField(field, nextValue);
      };

    return {
      setStatus: createFieldSetter('status'),
      setMode: createFieldSetter('mode'),
      setSession: createFieldSetter('session'),
      setStartPoint: createFieldSetter('startPoint'),
      setCursorPoint: createFieldSetter('cursorPoint'),
      setSelection: createFieldSetter('selection'),
      setHoverSelection: createFieldSetter('hoverSelection'),
      setEditGesture: createFieldSetter('editGesture'),
      setActiveAnnotationTool: createFieldSetter('activeAnnotationTool'),
      setAnnotationGesture: createFieldSetter('annotationGesture'),
      setDraftAnnotation: createFieldSetter('draftAnnotation'),
      setSelectedAnnotationIndex: createFieldSetter('selectedAnnotationIndex'),
      setAnnotationMoveGesture: createFieldSetter('annotationMoveGesture'),
      setDraftSelectionMoveGesture: createFieldSetter('draftSelectionMoveGesture'),
      setTextDraft: createFieldSetter('textDraft'),
      setTextDraftAnnotationIndex: createFieldSetter('textDraftAnnotationIndex'),
      setAnnotationStyle: createFieldSetter('annotationStyle'),
      setTextFontSize: createFieldSetter('textFontSize'),
      setAnnotationHistory: createFieldSetter('annotationHistory'),
      setPreviewImageBase64: createFieldSetter('previewImageBase64'),
      setIsAnnotationToolbarVisible: createFieldSetter(
        'isAnnotationToolbarVisible',
      ),
      setCursorColor: createFieldSetter('cursorColor'),
      setColorSampleFormat: createFieldSetter('colorSampleFormat'),
      setIsMagnifierRequested: createFieldSetter('isMagnifierRequested'),
      setIncludeCapturedCursor: createFieldSetter('includeCapturedCursor'),
      setError: createFieldSetter('error'),
    };
  }, [setWorkspaceField]);

  const workspaceActions = useMemo(
    () =>
      createCaptureWorkspaceStateActions({
        refs,
        applyPatch,
        onHoverSelectionSynced: options.onHoverSelectionSynced,
      }),
    [applyPatch, options.onHoverSelectionSynced, refs],
  );

  const resetInteraction = useCallback(() => {
    clearCaptureWorkspaceRefs(refs);
    applyPatch(resetCaptureInteractionStatePatch());
  }, [applyPatch, refs]);

  const resetSession = useCallback(() => {
    clearCaptureWorkspaceRefs(refs);
    applyPatch({
      status: 'idle',
      session: null,
      ...resetCaptureInteractionStatePatch(),
    });
  }, [applyPatch, refs]);

  const applyLoadedSession = useCallback(
    (loaded: LoadedCaptureHostSession) => {
      const patch = loadedCaptureHostSessionPatch(loaded);

      syncCaptureWorkspaceRefsFromPatch(refs, patch);
      applyPatch(patch);
    },
    [applyPatch, refs],
  );

  const resetPreview = useCallback(() => {
    clearCaptureWorkspaceRefs(refs);
    applyPatch(previewResetPatch());
  }, [applyPatch, refs]);

  return {
    ...state,
    ...fieldSetters,
    ...workspaceActions,
    refs,
    startPointRef,
    cursorPointRef,
    draftSelectionRef,
    hoverSelectionRef,
    applyPatch,
    resetInteraction,
    resetSession,
    applyLoadedSession,
    resetPreview,
  };
}

function resolveSetStateAction<Value>(
  currentValue: Value,
  nextValue: SetStateAction<Value>,
): Value {
  return typeof nextValue === 'function'
    ? (nextValue as (value: Value) => Value)(currentValue)
    : nextValue;
}

function areRectsEqual(a: LogicalRect | null, b: LogicalRect | null) {
  if (a === b) return true;
  if (!a || !b) return false;

  return (
    a.x === b.x &&
    a.y === b.y &&
    a.width === b.width &&
    a.height === b.height
  );
}
