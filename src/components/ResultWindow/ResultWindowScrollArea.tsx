import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
  type UIEvent,
} from 'react';

const scrollHideDelayMs = 900;
const scrollTrackInsetPx = 12;
const scrollThumbMinHeightPx = 24;

export interface ResultWindowScrollAreaMetricsInput {
  clientHeight: number;
  scrollHeight: number;
  scrollTop: number;
}

export interface ResultWindowScrollAreaMetrics {
  isScrollable: boolean;
  thumbHeightPx: number;
  thumbTopPx: number;
}

interface ResultWindowScrollAreaProps {
  className: string;
  children: ReactNode;
}

export function resultWindowScrollAreaIndicatorClassName(isVisible: boolean) {
  const visibilityClassName = isVisible
    ? 'translate-x-0 opacity-100 duration-200'
    : 'translate-x-1 opacity-0 duration-500';

  return `pointer-events-none absolute bottom-0 right-0.5 top-12 z-20 w-[3px] transition-[opacity,transform] ease-[cubic-bezier(0.16,1,0.3,1)] will-change-[opacity,transform] ${visibilityClassName}`;
}

export function resultWindowScrollAreaThumbClassName() {
  return 'absolute right-0 w-[3px] rounded-full bg-slate-400/35';
}

export function resultWindowScrollAreaMetrics({
  clientHeight,
  scrollHeight,
  scrollTop,
}: ResultWindowScrollAreaMetricsInput): ResultWindowScrollAreaMetrics {
  if (scrollHeight <= clientHeight || clientHeight <= 0) {
    return {
      isScrollable: false,
      thumbHeightPx: 0,
      thumbTopPx: scrollTrackInsetPx,
    };
  }

  const trackHeightPx = Math.max(0, clientHeight - scrollTrackInsetPx * 2);
  const scrollableDistancePx = scrollHeight - clientHeight;
  const thumbHeightPx = Math.min(
    trackHeightPx,
    Math.max(
      scrollThumbMinHeightPx,
      Math.round((clientHeight / scrollHeight) * trackHeightPx),
    ),
  );
  const thumbTravelPx = Math.max(0, trackHeightPx - thumbHeightPx);
  const scrollProgress = Math.min(
    1,
    Math.max(0, scrollTop / scrollableDistancePx),
  );

  return {
    isScrollable: true,
    thumbHeightPx,
    thumbTopPx: scrollTrackInsetPx + Math.round(scrollProgress * thumbTravelPx),
  };
}

export default function ResultWindowScrollArea({
  className,
  children,
}: ResultWindowScrollAreaProps) {
  const scrollAreaRef = useRef<HTMLDivElement>(null);
  const idleTimeoutRef = useRef<number | null>(null);
  const [isScrolling, setScrolling] = useState(false);
  const [metrics, setMetrics] = useState(() =>
    resultWindowScrollAreaMetrics({
      clientHeight: 0,
      scrollHeight: 0,
      scrollTop: 0,
    }),
  );

  const updateMetrics = useCallback((scrollElement = scrollAreaRef.current) => {
    if (!scrollElement) return;

    setMetrics(
      resultWindowScrollAreaMetrics({
        clientHeight: scrollElement.clientHeight,
        scrollHeight: scrollElement.scrollHeight,
        scrollTop: scrollElement.scrollTop,
      }),
    );
  }, []);

  const scheduleHide = useCallback(() => {
    if (idleTimeoutRef.current !== null) {
      window.clearTimeout(idleTimeoutRef.current);
    }

    idleTimeoutRef.current = window.setTimeout(() => {
      setScrolling(false);
      idleTimeoutRef.current = null;
    }, scrollHideDelayMs);
  }, []);

  const handleScroll = (event: UIEvent<HTMLDivElement>) => {
    updateMetrics(event.currentTarget);
    setScrolling(true);
    scheduleHide();
  };

  useLayoutEffect(() => {
    updateMetrics();
  }, [children, updateMetrics]);

  useEffect(() => {
    const scrollElement = scrollAreaRef.current;
    if (!scrollElement || typeof ResizeObserver === 'undefined') return;

    const resizeObserver = new ResizeObserver(() => updateMetrics(scrollElement));
    resizeObserver.observe(scrollElement);
    Array.from(scrollElement.children).forEach((child) => {
      resizeObserver.observe(child);
    });

    return () => resizeObserver.disconnect();
  }, [children, updateMetrics]);

  useEffect(() => {
    return () => {
      if (idleTimeoutRef.current !== null) {
        window.clearTimeout(idleTimeoutRef.current);
      }
    };
  }, []);

  return (
    <>
      <div ref={scrollAreaRef} className={className} onScroll={handleScroll}>
        {children}
      </div>

      {metrics.isScrollable && (
        <div
          className={resultWindowScrollAreaIndicatorClassName(isScrolling)}
          aria-hidden="true"
        >
          <div
            className={resultWindowScrollAreaThumbClassName()}
            style={{
              height: `${metrics.thumbHeightPx}px`,
              transform: `translateY(${metrics.thumbTopPx}px)`,
            }}
          />
        </div>
      )}
    </>
  );
}
