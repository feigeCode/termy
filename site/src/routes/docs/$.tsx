import { createFileRoute, Link, notFound, useNavigate } from "@tanstack/react-router";
import { getDocBySlug, getAllDocs, extractHeadings } from "@/lib/docs";
import { Sidebar } from "@/components/docs/Sidebar";
import { TableOfContents } from "@/components/docs/TableOfContents";
import Markdown from "react-markdown";
import type { Components } from "react-markdown";
import { useMemo, useCallback, type ReactNode } from "react";

type SearchParams = { q?: string };

export const Route = createFileRoute("/docs/$")({
  component: DocPage,
  validateSearch: (search: Record<string, unknown>): SearchParams => ({
    q: typeof search.q === "string" ? search.q : undefined,
  }),
  loader: ({ params }) => {
    const slug = params._splat ?? "";
    const doc = getDocBySlug(slug);
    if (!doc) {
      throw notFound();
    }
    return doc;
  },
});

// Generate heading ID from text
function generateId(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "");
}

// Highlight matching text
function highlightText(text: string, query: string): ReactNode {
  if (!query.trim()) return text;

  const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")})`, "gi");
  const parts = text.split(regex);

  if (parts.length === 1) return text;

  return parts.map((part, i) =>
    regex.test(part) ? (
      <mark key={i} className="bg-primary/30 text-foreground rounded px-0.5">
        {part}
      </mark>
    ) : (
      part
    )
  );
}

// Create markdown components with optional highlighting
function createMarkdownComponents(query: string): Components {
  const wrapChildren = (children: ReactNode): ReactNode => {
    if (!query.trim()) return children;

    if (typeof children === "string") {
      return highlightText(children, query);
    }

    if (Array.isArray(children)) {
      return children.map((child, i) =>
        typeof child === "string" ? (
          <span key={i}>{highlightText(child, query)}</span>
        ) : (
          child
        )
      );
    }

    return children;
  };

  return {
    h2: ({ children }) => {
      const text = String(children);
      const id = generateId(text);
      return (
        <h2 id={id} className="scroll-mt-24">
          {wrapChildren(children)}
        </h2>
      );
    },
    h3: ({ children }) => {
      const text = String(children);
      const id = generateId(text);
      return (
        <h3 id={id} className="scroll-mt-24">
          {wrapChildren(children)}
        </h3>
      );
    },
    h4: ({ children }) => {
      const text = String(children);
      const id = generateId(text);
      return (
        <h4 id={id} className="scroll-mt-24">
          {wrapChildren(children)}
        </h4>
      );
    },
    p: ({ children }) => <p>{wrapChildren(children)}</p>,
    li: ({ children }) => <li>{wrapChildren(children)}</li>,
    strong: ({ children }) => <strong>{wrapChildren(children)}</strong>,
    em: ({ children }) => <em>{wrapChildren(children)}</em>,
  };
}

function DocPage() {
  const doc = Route.useLoaderData();
  const { q: search = "" } = Route.useSearch();
  const navigate = useNavigate({ from: Route.fullPath });
  const allDocs = getAllDocs();
  const currentIndex = allDocs.findIndex((d) => d.slug === doc.slug);
  const prevDoc = currentIndex > 0 ? allDocs[currentIndex - 1] : null;
  const nextDoc =
    currentIndex < allDocs.length - 1 ? allDocs[currentIndex + 1] : null;
  const headings = extractHeadings(doc.content);

  const handleSearchChange = useCallback(
    (value: string) => {
      navigate({
        search: value ? { q: value } : {},
        replace: true,
      });
    },
    [navigate]
  );

  const markdownComponents = useMemo(
    () => createMarkdownComponents(search),
    [search]
  );

  return (
    <section className="pt-24 pb-20">
      <div className="flex gap-8">
        {/* Sidebar */}
        <Sidebar currentSlug={doc.slug} search={search} onSearchChange={handleSearchChange} />

        {/* Main content */}
        <main className="flex-1 min-w-0">
          {/* Mobile back link */}
          <Link
            to="/docs"
            className="lg:hidden text-sm text-muted-foreground hover:text-foreground transition-colors inline-flex items-center gap-1 mb-6"
          >
            <svg
              className="w-4 h-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
            All docs
          </Link>

          {/* Search indicator */}
          {search && (
            <div className="mb-4 flex items-center gap-2 text-sm text-muted-foreground">
              <span>Highlighting: </span>
              <span className="px-2 py-0.5 bg-primary/20 text-primary rounded font-medium">
                {search}
              </span>
            </div>
          )}

          {/* Header */}
          <div className="mb-8">
            {doc.category && (
              <span className="text-sm text-primary font-medium">
                {doc.category}
              </span>
            )}
            <h1 className="text-3xl md:text-4xl font-bold mt-1">{doc.title}</h1>
            {doc.description && (
              <p className="mt-3 text-muted-foreground">{doc.description}</p>
            )}
          </div>

          {/* Content */}
          <div className="prose prose-sm prose-invert max-w-none prose-headings:text-foreground prose-p:text-muted-foreground prose-strong:text-foreground prose-a:text-primary prose-a:no-underline hover:prose-a:underline prose-code:text-primary prose-code:bg-secondary prose-code:px-1.5 prose-code:py-0.5 prose-code:rounded prose-code:before:content-none prose-code:after:content-none prose-pre:bg-background prose-pre:border prose-pre:border-border/50 prose-ul:text-muted-foreground prose-li:marker:text-muted-foreground prose-li:text-muted-foreground">
            <Markdown components={markdownComponents}>{doc.content}</Markdown>
          </div>

          {/* Navigation */}
          <div className="mt-12 pt-8 border-t border-border/50 flex flex-col sm:flex-row justify-between gap-4">
            {prevDoc ? (
              <Link
                to="/docs/$"
                params={{ _splat: prevDoc.slug }}
                className="flex-1 p-4 rounded-xl border border-border/50 hover:border-primary/50 hover:bg-card/30 transition-all group"
              >
                <span className="text-xs text-muted-foreground">Previous</span>
                <div className="flex items-center gap-2 mt-1">
                  <svg
                    className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M15 19l-7-7 7-7"
                    />
                  </svg>
                  <span className="font-medium text-foreground group-hover:text-primary transition-colors">
                    {prevDoc.title}
                  </span>
                </div>
              </Link>
            ) : (
              <div className="flex-1" />
            )}

            {nextDoc ? (
              <Link
                to="/docs/$"
                params={{ _splat: nextDoc.slug }}
                className="flex-1 p-4 rounded-xl border border-border/50 hover:border-primary/50 hover:bg-card/30 transition-all group sm:text-right"
              >
                <span className="text-xs text-muted-foreground">Next</span>
                <div className="flex items-center sm:justify-end gap-2 mt-1">
                  <span className="font-medium text-foreground group-hover:text-primary transition-colors">
                    {nextDoc.title}
                  </span>
                  <svg
                    className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 5l7 7-7 7"
                    />
                  </svg>
                </div>
              </Link>
            ) : (
              <div className="flex-1" />
            )}
          </div>
        </main>

        {/* Table of contents */}
        <TableOfContents headings={headings} />
      </div>
    </section>
  );
}
