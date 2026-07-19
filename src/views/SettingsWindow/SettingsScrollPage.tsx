import { useEffect, useRef, useState, type ReactNode } from 'react';

export interface SettingsScrollSection {
  id: string;
  label: string;
  description: string;
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
  description: string;
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
    <div ref={scrollRef} className="h-full flex-1 overflow-y-auto bg-[#f4f5f7]">
      <main className="mx-auto w-full max-w-[1080px] px-8 pb-16">
        <header className="sticky top-0 z-20 -mx-[22px] mb-[18px] flex items-center justify-between gap-7 border-b border-gray-200/95 bg-[#f4f5f7]/95 px-[22px] pb-[15px] pt-[22px] backdrop-blur-xl">
          <div className="min-w-0">
            <div className="flex items-center gap-2.5">
              <h1 className="text-[26px] font-bold tracking-[-0.045em] text-gray-950">
                {title}
              </h1>
              <span className="flex items-center gap-1.5 text-[10px] font-medium text-gray-500">
                <i className="h-1.5 w-1.5 rounded-full bg-green-500 ring-[3px] ring-green-100" />
                {autoSaveLabel}
              </span>
            </div>
            <p className="mt-1 text-[12px] text-gray-500">{description}</p>
          </div>

          <nav className="flex max-w-[68%] shrink overflow-x-auto rounded-[9px] border border-gray-200 bg-white p-1 shadow-sm [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
            {sections.map((section) => {
              const active = activeSection === section.id;
              return (
                <button
                  key={section.id}
                  type="button"
                  aria-current={active ? 'page' : undefined}
                  onClick={() => jumpToSection(section.id)}
                  className={`h-[30px] shrink-0 rounded-md px-3 text-[11px] font-semibold transition-colors ${
                    active
                      ? 'bg-primary-50 text-primary-700'
                      : 'text-gray-500 hover:bg-gray-50 hover:text-gray-800'
                  }`}
                >
                  {section.label}
                </button>
              );
            })}
          </nav>
        </header>

        <div className="space-y-4">
          {sections.map((section) => (
            <section
              key={section.id}
              ref={(node) => {
                sectionRefs.current[section.id] = node;
              }}
              className="-mx-[22px] scroll-mt-[94px] overflow-visible rounded-[11px] border border-gray-200 bg-white shadow-sm"
            >
              <header className="flex items-center justify-between gap-5 rounded-t-[10px] border-b border-gray-100 bg-gray-50/30 px-[22px] py-4">
                <div className="min-w-0">
                  <h2 className="text-base font-semibold tracking-[-0.02em] text-gray-900">
                    {section.label}
                  </h2>
                  <p className="mt-1 text-[11px] leading-relaxed text-gray-500">
                    {section.description}
                  </p>
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
