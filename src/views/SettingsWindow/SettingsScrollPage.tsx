import { useEffect, useRef, useState, type ReactNode } from 'react';

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
}: {
  title: string;
  description?: string;
  sections: SettingsScrollSection[];
  requestedSectionId?: string | null;
  onRequestedSectionHandled?: () => void;
  autoSaveLabel?: string;
}) {
  const [activeSection, setActiveSection] = useState(sections[0]?.id ?? '');
  const scrollRef = useRef<HTMLDivElement>(null);
  const headerRef = useRef<HTMLElement>(null);
  const sectionRefs = useRef<Record<string, HTMLElement | null>>({});

  useEffect(() => {
    const container = scrollRef.current;
    if (!container || sections.length === 0) return;

    const updateActiveSection = () => {
      const threshold = (headerRef.current?.getBoundingClientRect().bottom ?? container.getBoundingClientRect().top) + 16;
      let nextSection = sections[0].id;
      for (const section of sections) {
        if (
          (sectionRefs.current[section.id]?.getBoundingClientRect().top ?? Infinity) <=
          threshold
        ) {
          nextSection = section.id;
        }
      }
      setActiveSection((current) =>
        current === nextSection ? current : nextSection,
      );
    };

    const syncHeader = () => {
      container.style.scrollPaddingTop = `${headerRef.current?.offsetHeight ?? 0}px`;
      updateActiveSection();
    };
    const observer = new ResizeObserver(syncHeader);
    if (headerRef.current) observer.observe(headerRef.current);
    container.addEventListener('scroll', updateActiveSection, { passive: true });
    syncHeader();
    return () => {
      observer.disconnect();
      container.removeEventListener('scroll', updateActiveSection);
    };
  }, [sections]);

  useEffect(() => {
    if (!requestedSectionId) return;
    const section = sectionRefs.current[requestedSectionId];
    if (!section) return;

    setActiveSection(requestedSectionId);
    section.scrollIntoView({ behavior: 'smooth', block: 'start' });
    onRequestedSectionHandled?.();
  }, [onRequestedSectionHandled, requestedSectionId]);

  const jumpToSection = (sectionId: string) => {
    setActiveSection(sectionId);
    sectionRefs.current[sectionId]?.scrollIntoView({
      behavior: 'smooth',
      block: 'start',
    });
  };

  const sectionButtons = sections.map((section, index) => (
    <button
      key={section.id}
      type="button"
      aria-current={activeSection === section.id ? 'page' : undefined}
      onClick={() => jumpToSection(section.id)}
      className={`settings-section-link h-[25px] shrink-0 whitespace-nowrap rounded-sm px-2.5 text-label font-medium transition-colors ${
        activeSection === section.id ? 'bg-surface text-ink shadow-xs' : 'text-muted hover:text-ink'
      }`}
    >
      <span className="settings-section-number" aria-hidden="true">{String(index + 1).padStart(2, '0')}</span>
      <span>{section.label}</span>
    </button>
  ));

  return (
    <div className="settings-scroll-layout">
      <aside className="settings-context-nav">
        <div className="settings-context-title">{title}<span>CONTENTS</span></div>
        <nav aria-label={`${title}目录`}>{sectionButtons}</nav>
        <span className="settings-context-status">{autoSaveLabel}</span>
      </aside>
    <div ref={scrollRef} className="settings-scroll-area h-full min-w-0 flex-1 overflow-y-auto bg-canvas">
      <main className="settings-page mx-auto w-full max-w-[1080px] px-8 pb-16">
        <header ref={headerRef} className="settings-page-header sticky top-0 z-20 -mx-[22px] mb-[18px] flex items-center justify-between gap-7 border-b border-line bg-canvas/90 px-[22px] pb-[15px] pt-[22px] backdrop-blur-xl">
          <div className="min-w-0">
            <div className="flex items-center gap-2.5">
              <h1 className="text-display font-bold tracking-[-0.028em] text-ink">
                {title}
              </h1>
              <span className="settings-save-status flex items-center gap-1.5 text-hint font-medium text-muted">
                <i className="h-1.5 w-1.5 rounded-full bg-good ring-[3px] ring-good-soft" />
                {autoSaveLabel}
              </span>
            </div>
            {description && (
              <p className="mt-1 text-label text-muted">{description}</p>
            )}
          </div>

          <nav aria-label={`${title}分组`} className="settings-section-tabs flex max-w-[60%] shrink gap-0.5 overflow-x-auto rounded-md bg-raised p-[3px] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            {sectionButtons}
          </nav>
        </header>

        <div className="settings-card-stack space-y-3">
          {sections.map((section, index) => (
            <section
              key={section.id}
              ref={(node) => {
                sectionRefs.current[section.id] = node;
              }}
              className="settings-card -mx-[22px] scroll-mt-4 overflow-visible rounded-lg border border-line bg-surface shadow-e1"
            >
              <header className="settings-card-heading flex items-center justify-between gap-5 rounded-t-lg border-b border-line bg-sunken px-[22px] py-3">
                <div className="min-w-0">
                  <h2 className="text-card font-semibold tracking-[-0.012em] text-ink">
                    <span className="settings-section-number" aria-hidden="true">{String(index + 1).padStart(2, '0')}</span>
                    {section.label}
                  </h2>
                  {section.description && (
                    <p className="mt-0.5 text-hint leading-relaxed text-muted">
                      {section.description}
                    </p>
                  )}
                </div>
                {section.action}
              </header>
              {section.content}
            </section>
          ))}
        </div>
      </main>
    </div>
    </div>
  );
}
