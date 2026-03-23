import { useEffect, useState } from "react";
import type { Heading } from "@/lib/docs";

interface TableOfContentsProps {
  headings: Heading[];
}

export function TableOfContents({ headings }: TableOfContentsProps) {
  const [activeId, setActiveId] = useState<string>("");

  useEffect(() => {
    if (headings.length === 0) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const visibleEntries = entries.filter((entry) => entry.isIntersecting);

        if (visibleEntries.length > 0) {
          const topEntry = visibleEntries.reduce((prev, current) => {
            return prev.boundingClientRect.top < current.boundingClientRect.top
              ? prev
              : current;
          });
          setActiveId(topEntry.target.id);
        }
      },
      {
        rootMargin: "-100px 0px -70% 0px",
        threshold: 0,
      }
    );

    headings.forEach((heading) => {
      const element = document.getElementById(heading.id);
      if (element) {
        observer.observe(element);
      }
    });

    return () => observer.disconnect();
  }, [headings]);

  if (headings.length === 0) {
    return null;
  }

  return (
    <aside className="hidden xl:block w-56 shrink-0">
      <nav className="sticky top-24">
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-4">
          On this page
        </h4>
        <ul className="space-y-2 border-l border-border/50">
          {headings.map((heading) => (
            <li key={heading.id}>
              <a
                href={`#${heading.id}`}
                className={`block text-sm py-1 transition-colors border-l-2 -ml-[2px] pl-4 ${
                  heading.level === 3 ? "ml-3" : heading.level === 4 ? "ml-6" : ""
                } ${
                  activeId === heading.id
                    ? "border-primary text-primary font-medium"
                    : "border-transparent text-muted-foreground hover:text-foreground hover:border-border"
                }`}
              >
                {heading.text}
              </a>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  );
}
