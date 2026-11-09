import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ButtonHTMLAttributes,
  type FocusEvent,
  type MouseEvent,
  type ReactNode,
} from 'react';
import HoverTooltip from './HoverTooltip';

interface IconActionButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'title'> {
  title: string;
  children: ReactNode;
  tooltipPlacement?: 'top' | 'bottom';
}

export default function IconActionButton({
  title,
  children,
  className = '',
  tooltipPlacement = 'top',
  ...buttonProps
}: IconActionButtonProps) {
  const buttonRef = useRef<HTMLButtonElement>(null);
  const hoverDelayTimerRef = useRef<number | null>(null);
  const [isTooltipVisible, setIsTooltipVisible] = useState(false);
  const [tooltipAnchorRect, setTooltipAnchorRect] = useState<DOMRect | null>(null);

  const clearHoverDelayTimer = useCallback(() => {
    if (hoverDelayTimerRef.current === null || typeof window === 'undefined') {
      return;
    }

    window.clearTimeout(hoverDelayTimerRef.current);
    hoverDelayTimerRef.current = null;
  }, []);

  const updateTooltipAnchorRect = useCallback(() => {
    const rect = buttonRef.current?.getBoundingClientRect();
    if (!rect) return;

    setTooltipAnchorRect({
      x: rect.x,
      y: rect.y,
      top: rect.top,
      right: rect.right,
      bottom: rect.bottom,
      left: rect.left,
      width: rect.width,
      height: rect.height,
      toJSON: () => rect.toJSON(),
    } as DOMRect);
  }, []);

  const showTooltipImmediately = useCallback(() => {
    if (buttonProps.disabled) return;

    clearHoverDelayTimer();
    updateTooltipAnchorRect();
    setIsTooltipVisible(true);
  }, [buttonProps.disabled, clearHoverDelayTimer, updateTooltipAnchorRect]);

  const showTooltipWithDelay = useCallback(() => {
    if (buttonProps.disabled || typeof window === 'undefined') return;

    clearHoverDelayTimer();
    updateTooltipAnchorRect();
    hoverDelayTimerRef.current = window.setTimeout(() => {
      updateTooltipAnchorRect();
      setIsTooltipVisible(true);
      hoverDelayTimerRef.current = null;
    }, 320);
  }, [buttonProps.disabled, clearHoverDelayTimer, updateTooltipAnchorRect]);

  const hideTooltip = useCallback(() => {
    clearHoverDelayTimer();
    setIsTooltipVisible(false);
  }, [clearHoverDelayTimer]);

  useEffect(() => {
    if (!isTooltipVisible || typeof window === 'undefined') {
      return;
    }

    const handleViewportChange = () => {
      updateTooltipAnchorRect();
    };

    window.addEventListener('resize', handleViewportChange);
    window.addEventListener('scroll', handleViewportChange, true);

    return () => {
      window.removeEventListener('resize', handleViewportChange);
      window.removeEventListener('scroll', handleViewportChange, true);
    };
  }, [isTooltipVisible, updateTooltipAnchorRect]);

  useEffect(
    () => () => {
      clearHoverDelayTimer();
    },
    [clearHoverDelayTimer],
  );

  const handleMouseEnter = (event: MouseEvent<HTMLButtonElement>) => {
    showTooltipWithDelay();
    buttonProps.onMouseEnter?.(event);
  };

  const handleMouseLeave = (event: MouseEvent<HTMLButtonElement>) => {
    hideTooltip();
    buttonProps.onMouseLeave?.(event);
  };

  const handleFocus = (event: FocusEvent<HTMLButtonElement>) => {
    showTooltipImmediately();
    buttonProps.onFocus?.(event);
  };

  const handleBlur = (event: FocusEvent<HTMLButtonElement>) => {
    hideTooltip();
    buttonProps.onBlur?.(event);
  };

  return (
    <button
      ref={buttonRef}
      type="button"
      aria-label={buttonProps['aria-label'] ?? title}
      {...buttonProps}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onFocus={handleFocus}
      onBlur={handleBlur}
      className={`group relative ${className}`}
    >
      {children}
      <HoverTooltip
        label={title}
        placement={tooltipPlacement}
        visible={isTooltipVisible}
        anchorRect={tooltipAnchorRect}
      />
    </button>
  );
}
