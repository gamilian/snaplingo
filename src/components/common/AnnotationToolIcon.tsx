export function AnnotationToolIcon({ tool }: { tool: string }) {
  const paths: Record<string, string> = {
    arrow: 'm5.5 18.5 13-13M11.8 5.5h6.7v6.7',
    line: 'm5.5 18.5 13-13',
    pen: 'm5 19 1.15-4.45 9.15-9.15 3.3 3.3-9.15 9.15L5 19Zm8.9-12.2 3.3 3.3',
    highlight: 'm6 14 8-8 4 4-8 8-4-4Zm-2 6 3-4 3 3-4 3',
    text: 'M6.5 5.5h11M12 5.5v13M8.8 18.5h6.4',
    eraser: 'm5 15.2 8.7-8.7 4.8 4.8-7.2 7.2H8.2L5 15.2Zm5.1-5.1 4.8 4.8M11.3 18.5h7.2',
    scrolling: 'M8 3H6v18h12V3h-2M12 6v12m-3-3 3 3 3-3',
  };
  return <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.65" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {tool === 'rectangle' && <rect x="4.5" y="5.5" width="15" height="13" rx="1.5" />}
    {tool === 'ellipse' && <ellipse cx="12" cy="12" rx="8" ry="6.5" />}
    {tool === 'mosaic' && <><rect x="4.5" y="4.5" width="15" height="15" rx="1.3" /><path d="M9.5 4.5v15M14.5 4.5v15M4.5 9.5h15M4.5 14.5h15" /></>}
    {tool === 'blur' && <><circle cx="12" cy="12" r="8" strokeDasharray="2 2" /><circle cx="12" cy="12" r="3" fill="currentColor" opacity=".3" /></>}
    {tool === 'spotlight' && <><circle cx="12" cy="12" r="4" /><path d="M12 2v3m0 14v3M2 12h3m14 0h3" /></>}
    {paths[tool] && <path d={paths[tool]} />}
  </svg>;
}
