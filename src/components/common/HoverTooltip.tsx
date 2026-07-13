import { createPortal } from 'react-dom';

interface HoverTooltipProps {
  label: string;
  placement?: 'top' | 'bottom';
  visible?: boolean;
  anchorRect?: DOMRect | null;
}

export default function HoverTooltip({
  label,
  placement = 'top',
  visible = false,
  anchorRect = null,
}: HoverTooltipProps) {
  if (!visible || !anchorRect || typeof document === 'undefined' || !document.body) {
    return null;
  }

  const gapPx = placement === 'bottom' ? 16 : 10;
  const left = anchorRect.left + anchorRect.width / 2;
  const top = placement === 'bottom' ? anchorRect.bottom + gapPx : anchorRect.top - gapPx;
  const tooltipStyle = {
    left: `${left}px`,
    top: `${top}px`,
    transform:
      placement === 'bottom' ? 'translate(-50%, 0)' : 'translate(-50%, -100%)',
  } as const;
  const arrowClassName =
    placement === 'bottom'
      ? '-top-[5px] border-l border-t border-slate-200/90'
      : '-bottom-[5px] border-b border-r border-slate-200/90';

  return createPortal(
    <span
      aria-hidden="true"
      style={tooltipStyle}
      className="pointer-events-none fixed z-[2147483647] whitespace-nowrap"
    >
      <span className="relative block rounded-[10px] border border-slate-200/90 bg-white/95 px-2.5 py-1.5 text-[11px] font-medium leading-none text-slate-700 shadow-[0_12px_30px_rgba(15,23,42,0.14),0_2px_6px_rgba(15,23,42,0.06)] backdrop-blur-md">
        <span
          aria-hidden="true"
          className={`absolute left-1/2 h-[10px] w-[10px] -translate-x-1/2 rotate-45 bg-white/95 ${arrowClassName}`}
        />
        <span className="relative block">{label}</span>
      </span>
    </span>,
    document.body,
  );
}
