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
  const sectionRefs = useRef<Record<string, HTMLElement | null>>({});

  useEffect(() => {
    const container = scrollRef.current;
    if (!container || sections.length === 0) return;

    const updateActiveSection = () => {
      const threshold = container.getBoundingClientRect().top + 112;
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

    container.addEventListener('scroll', updateActiveSection, { passive: true });
    updateActiveSection();
    return () => container.removeEventListener('scroll', updateActiveSection);
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

  return (
    <div ref={scrollRef} className="h-full flex-1 overflow-y-auto bg-canvas">
      <main className="mx-auto w-full max-w-[1080px] px-8 pb-16">
        <header className="sticky top-0 z-20 -mx-[22px] mb-[18px] flex items-center justify-between gap-7 border-b border-line bg-canvas/90 px-[22px] pb-[15px] pt-[22px] backdrop-blur-xl">
          <div className="min-w-0">
            <div className="flex items-center gap-2.5">
              <h1 className="text-display font-bold tracking-[-0.028em] text-ink">
                {title}
              </h1>
              <span className="flex items-center gap-1.5 text-hint font-medium text-muted">
                <i className="h-1.5 w-1.5 rounded-full bg-good ring-[3px] ring-good-soft" />
                {autoSaveLabel}
              </span>
            </div>
            {description && (
              <p className="mt-1 text-label text-muted">{description}</p>
            )}
          </div>

          <nav className="flex max-w-[60%] shrink gap-0.5 overflow-x-auto rounded-md bg-raised p-[3px] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            {sections.map((section) => {
              const active = activeSection === section.id;
              return (
                <button
                  key={section.id}
                  type="button"
                  aria-current={active ? 'page' : undefined}
                  onClick={() => jumpToSection(section.id)}
                  className={`h-[25px] shrink-0 whitespace-nowrap rounded-sm px-2.5 text-label font-medium transition-colors ${
                    active
                      ? 'bg-surface text-ink shadow-xs'
                      : 'text-muted hover:text-ink'
                  }`}
                >
                  {section.label}
                </button>
              );
            })}
          </nav>
        </header>

        <div className="space-y-3">
          {sections.map((section) => (
            <section
              key={section.id}
              ref={(node) => {
                sectionRefs.current[section.id] = node;
              }}
              className="-mx-[22px] scroll-mt-[94px] overflow-visible rounded-lg border border-line bg-surface shadow-e1"
            >
              <header className="flex items-center justify-between gap-5 rounded-t-lg border-b border-line bg-sunken px-[22px] py-3">
                <div className="min-w-0">
                  <h2 className="text-card font-semibold tracking-[-0.012em] text-ink">
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
  );
}
