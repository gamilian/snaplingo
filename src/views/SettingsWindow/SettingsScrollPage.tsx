import { Fragment, useEffect, useRef, type ReactNode } from 'react';

export interface SettingsScrollSection {
  id: string;
  label: string;
  description?: string;
  content: ReactNode;
  action?: ReactNode;
}

export function SettingsScrollPage({
  title,
  description,
  sections,
  requestedSectionId,
  onRequestedSectionHandled,
  autoSaveLabel = '自动保存',
  intro,
  actions,
  contentOnly = false,
}: {
  title: string;
  description?: string;
  sections: SettingsScrollSection[];
  requestedSectionId?: string | null;
  onRequestedSectionHandled?: () => void;
  autoSaveLabel?: string;
  intro?: ReactNode;
  actions?: ReactNode;
  contentOnly?: boolean;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<Record<string, HTMLElement | null>>({});

  useEffect(() => {
    if (!requestedSectionId) return;
    const section = sectionRefs.current[requestedSectionId];
    if (!section) return;
    section.scrollIntoView({ behavior: 'smooth', block: 'start' });
    onRequestedSectionHandled?.();
  }, [onRequestedSectionHandled, requestedSectionId]);

  const content = (
    <div className="settings-card-stack">
      {sections.map((section) => (
        <section
          key={section.id}
          ref={(node) => { sectionRefs.current[section.id] = node; }}
          className="settings-section overflow-visible"
          data-settings-section={section.id}
        >
          <header className="settings-card-heading">
            <div className="min-w-0">
              <h2>{section.label}</h2>
              {section.description && <p className="mt-1 text-hint text-muted">{section.description}</p>}
            </div>
            {section.action}
          </header>
          <div className="settings-card">{section.content}</div>
        </section>
      ))}
    </div>
  );

  if (contentOnly) return <>{sections.map((section) => <Fragment key={section.id}>{section.content}</Fragment>)}</>;

  return (
    <div className="settings-scroll-layout">
      <header className="settings-page-header">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-2.5">
            <h1>{title}</h1>
            <span className="settings-save-status">● {autoSaveLabel}</span>
          </div>
          {description && <p className="mt-1 text-xs text-muted">{description}</p>}
        </div>
        {actions}
      </header>
      <div ref={scrollRef} className="settings-scroll-area">
        <main className="settings-page">
          {intro}
          {content}
        </main>
      </div>
    </div>
  );
}
