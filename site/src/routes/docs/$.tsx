import { createFileRoute, Link, notFound } from "@tanstack/react-router";
import { ArrowLeft, ArrowRight, ChevronLeft } from "lucide-react";
import rehypeHighlight from "rehype-highlight";
import { useMemo, type JSX, type ReactNode } from "react";
import Markdown from "react-markdown";
import type { Components } from "react-markdown";
import { CopyButton } from "@/components/animate-ui/components/buttons/copy";
import { Sidebar } from "@/components/docs/Sidebar";
import { TableOfContents } from "@/components/docs/TableOfContents";
import { Button } from "@/components/ui/button";
import { validateSearch, useDocSearchChange } from "@/hooks/useDocSearch";
import {
  extractHeadings,
  getAllDocs,
  getDocBySlug,
  type Doc,
} from "@/lib/docs";
import { generateSlug, proseClasses } from "@/lib/utils";

export const Route = createFileRoute("/docs/$")({
  component: DocPage,
  validateSearch,
  loader: ({ params }) => {
    const slug = params._splat ?? "";
    const doc = getDocBySlug(slug);

    if (!doc) {
      throw notFound();
    }

    return doc;
  },
});

interface AdjacentDocs {
  prevDoc: Doc | null;
  nextDoc: Doc | null;
}

function getAdjacentDocs(currentSlug: string, docs: Doc[]): AdjacentDocs {
  const currentIndex = docs.findIndex((doc) => doc.slug === currentSlug);

  if (currentIndex === -1) {
    return { prevDoc: null, nextDoc: null };
  }

  const prevDoc = currentIndex > 0 ? docs[currentIndex - 1] : null;
  const nextDoc =
    currentIndex < docs.length - 1 ? docs[currentIndex + 1] : null;

  return { prevDoc, nextDoc };
}

function escapeRegex(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function highlightText(text: string, query: string): ReactNode {
  const trimmedQuery = query.trim();
  if (!trimmedQuery) {
    return text;
  }

  const regex = new RegExp(`(${escapeRegex(trimmedQuery)})`, "gi");
  const parts = text.split(regex);

  if (parts.length === 1) {
    return text;
  }

  return parts.map((part, index) => {
    if (index % 2 === 0) {
      return part;
    }

    return (
      <mark
        key={index}
        className="bg-primary/25 text-primary rounded px-1 py-0.5 font-medium"
      >
        {part}
      </mark>
    );
  });
}

function wrapHighlightedChildren(
  children: ReactNode,
  query: string,
): ReactNode {
  if (!query.trim()) {
    return children;
  }

  if (typeof children === "string") {
    return highlightText(children, query);
  }

  if (Array.isArray(children)) {
    return children.map((child, index) => {
      if (typeof child !== "string") {
        return child;
      }

      return <span key={index}>{highlightText(child, query)}</span>;
    });
  }

  return children;
}

function renderHeading(
  tagName: "h2" | "h3" | "h4",
  children: ReactNode,
  query: string,
): JSX.Element {
  const id = generateSlug(String(children));
  const content = wrapHighlightedChildren(children, query);

  const baseClasses = "scroll-mt-28 font-semibold tracking-tight";

  if (tagName === "h2") {
    return (
      <h2 id={id} className={`${baseClasses} text-2xl mt-12 mb-4 text-foreground`}>
        {content}
      </h2>
    );
  }

  if (tagName === "h3") {
    return (
      <h3 id={id} className={`${baseClasses} text-xl mt-8 mb-3 text-foreground`}>
        {content}
      </h3>
    );
  }

  return (
    <h4 id={id} className={`${baseClasses} text-lg mt-6 mb-2 text-foreground`}>
      {content}
    </h4>
  );
}

function createMarkdownComponents(query: string): Components {
  return {
    h2: ({ children }) => renderHeading("h2", children, query),
    h3: ({ children }) => renderHeading("h3", children, query),
    h4: ({ children }) => renderHeading("h4", children, query),
    p: ({ children }) => (
      <p className="leading-7 mb-4">{wrapHighlightedChildren(children, query)}</p>
    ),
    li: ({ children }) => (
      <li className="mb-1">{wrapHighlightedChildren(children, query)}</li>
    ),
    strong: ({ children }) => (
      <strong className="font-semibold text-foreground">{wrapHighlightedChildren(children, query)}</strong>
    ),
    em: ({ children }) => (
      <em className="italic">{wrapHighlightedChildren(children, query)}</em>
    ),
    code: ({ children, className }) => {
      const isInline = !className;
      if (isInline) {
        return (
          <code className="px-1.5 py-0.5 rounded-md bg-secondary text-sm font-mono text-primary">
            {children}
          </code>
        );
      }
      return <code className={className}>{children}</code>;
    },
    pre: ({ children }) => {
      let content = "";
      if (
        children &&
        typeof children === "object" &&
        "props" in children &&
        children.props &&
        typeof children.props === "object" &&
        "children" in children.props
      ) {
        const codeChildren = children.props.children;
        if (typeof codeChildren === "string") {
          content = codeChildren;
        } else if (Array.isArray(codeChildren)) {
          content = codeChildren.join("");
        }
      }

      return (
        <div className="relative group my-6">
          <CopyButton
            content={content.replace(/\n$/, "")}
            variant="outline"
            size="xs"
            className="absolute right-3 top-3 z-10 opacity-0 transition-opacity group-hover:opacity-100"
            aria-label="Copy code"
            title="Copy code"
          />
          <pre className="rounded-xl bg-card/80 border border-border/50 p-4 overflow-x-auto">
            {children}
          </pre>
        </div>
      );
    },
    blockquote: ({ children }) => (
      <blockquote className="border-l-2 border-primary/50 pl-4 italic text-muted-foreground my-6">
        {children}
      </blockquote>
    ),
    hr: () => <hr className="border-border/50 my-8" />,
  };
}

function DocPage(): JSX.Element {
  const doc = Route.useLoaderData();
  const { q: search = "" } = Route.useSearch();
  const allDocs = getAllDocs();
  const { prevDoc, nextDoc } = getAdjacentDocs(doc.slug, allDocs);
  const headings = extractHeadings(doc.content);
  const handleSearchChange = useDocSearchChange(Route.fullPath);

  const markdownComponents = useMemo(
    () => createMarkdownComponents(search),
    [search],
  );

  return (
    <section className="relative min-h-screen">
      {/* Ambient background */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-1/4 right-0 w-[500px] h-[500px] bg-primary/5 rounded-full blur-[120px]" />
      </div>

      <div className="relative pt-24 pb-20">
        <div className="flex gap-10">
          <Sidebar
            currentSlug={doc.slug}
            search={search}
            onSearchChange={handleSearchChange}
          />

          <main className="flex-1 min-w-0">
            {/* Mobile navigation */}
            <div className="lg:hidden mb-6 flex items-center gap-4">
              <Button
                asChild
                variant="ghost"
                size="sm"
                className="text-muted-foreground hover:text-foreground"
              >
                <Link to="/docs">
                  <ChevronLeft className="w-4 h-4 mr-1" />
                  All docs
                </Link>
              </Button>
            </div>

            {/* Mobile search */}
            <div className="lg:hidden mb-6">
              <SearchInput value={search} onChange={handleSearchChange} />
            </div>

            {/* Search indicator */}
            {search && (
              <div className="mb-6 flex items-center gap-2 text-sm">
                <span className="text-muted-foreground">Highlighting:</span>
                <span className="px-2 py-0.5 bg-primary/15 text-primary rounded-md font-medium">
                  {search}
                </span>
              </div>
            )}

            {/* Article header */}
            <article>
              <header className="mb-10">
                {doc.category && (
                  <span className="inline-block text-sm font-medium text-primary mb-3">
                    {doc.category}
                  </span>
                )}
                <h1 className="text-3xl md:text-4xl font-bold tracking-tight text-foreground">
                  {doc.title}
                </h1>
                {doc.description && (
                  <p className="mt-4 text-lg text-muted-foreground leading-relaxed">
                    {doc.description}
                  </p>
                )}
              </header>

              {/* Article content */}
              <div className={`${proseClasses} max-w-none`}>
                <Markdown
                  components={markdownComponents}
                  rehypePlugins={[rehypeHighlight]}
                >
                  {doc.content}
                </Markdown>
              </div>
            </article>

            {/* Pagination */}
            <nav className="mt-16 pt-8 border-t border-border/50">
              <div className="flex flex-col sm:flex-row justify-between gap-4">
                {prevDoc ? (
                  <Link
                    to="/docs/$"
                    params={{ _splat: prevDoc.slug }}
                    className="group flex-1 p-5 rounded-xl border border-border/50 hover:border-primary/30 hover:bg-card/30 transition-all"
                  >
                    <span className="text-xs text-muted-foreground uppercase tracking-wider">
                      Previous
                    </span>
                    <div className="flex items-center gap-2 mt-1">
                      <ArrowLeft className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors" />
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
                    className="group flex-1 p-5 rounded-xl border border-border/50 hover:border-primary/30 hover:bg-card/30 transition-all sm:text-right"
                  >
                    <span className="text-xs text-muted-foreground uppercase tracking-wider">
                      Next
                    </span>
                    <div className="flex items-center sm:justify-end gap-2 mt-1">
                      <span className="font-medium text-foreground group-hover:text-primary transition-colors">
                        {nextDoc.title}
                      </span>
                      <ArrowRight className="w-4 h-4 text-muted-foreground group-hover:text-primary transition-colors" />
                    </div>
                  </Link>
                ) : (
                  <div className="flex-1" />
                )}
              </div>
            </nav>
          </main>

          <TableOfContents headings={headings} />
        </div>
      </div>
    </section>
  );
}

function SearchInput({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <div className="relative">
      <svg
        className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
        />
      </svg>
      <input
        type="text"
        placeholder="Search documentation..."
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="w-full pl-9 pr-8 py-2.5 text-sm bg-card/50 border border-border/50 rounded-xl placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-primary/30 focus:border-primary/30 transition-all"
      />
      {value && (
        <button
          type="button"
          onClick={() => onChange("")}
          className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-muted-foreground hover:text-foreground transition-colors"
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
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      )}
    </div>
  );
}
