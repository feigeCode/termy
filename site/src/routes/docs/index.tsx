import { createFileRoute, Link } from "@tanstack/react-router";
import { ArrowRight, BookOpen, Sparkles, Terminal, Wrench } from "lucide-react";
import type { JSX } from "react";
import { Sidebar } from "@/components/docs/Sidebar";
import { Button } from "@/components/ui/button";
import { validateSearch, useDocSearchChange } from "@/hooks/useDocSearch";
import { getDocsByCategory, sortDocCategories } from "@/lib/docs";

const START_HERE_DOCS = [
  { slug: "installation", label: "Install Termy", icon: Terminal },
  { slug: "first-steps", label: "First Steps", icon: Sparkles },
  { slug: "troubleshooting", label: "Troubleshooting", icon: Wrench },
] as const;

export const Route = createFileRoute("/docs/")({
  component: DocsPage,
  validateSearch,
});

function DocsPage(): JSX.Element {
  const { q: search = "" } = Route.useSearch();
  const docsByCategory = getDocsByCategory();
  const categories = sortDocCategories(Object.keys(docsByCategory));
  const handleSearchChange = useDocSearchChange(Route.fullPath);

  return (
    <section className="relative min-h-screen">
      {/* Ambient background */}
      <div className="fixed inset-0 pointer-events-none">
        <div className="absolute top-0 left-1/4 w-[600px] h-[600px] bg-primary/5 rounded-full blur-[120px]" />
        <div className="absolute bottom-0 right-1/4 w-[400px] h-[400px] bg-primary/3 rounded-full blur-[100px]" />
      </div>

      <div className="relative pt-24 pb-20">
        <div className="flex gap-10">
          {/* Sidebar */}
          <Sidebar
            currentSlug=""
            search={search}
            onSearchChange={handleSearchChange}
          />

          {/* Main content */}
          <main className="flex-1 min-w-0">
            {/* Mobile back link */}
            <Button
              asChild
              variant="ghost"
              size="sm"
              className="lg:hidden mb-6 text-muted-foreground hover:text-foreground"
            >
              <Link to="/">
                <ArrowRight className="w-4 h-4 rotate-180 mr-1" />
                Back to home
              </Link>
            </Button>

            {/* Mobile search */}
            <div className="lg:hidden mb-8">
              <SearchInput value={search} onChange={handleSearchChange} />
            </div>

            {/* Hero section */}
            <div className="mb-12">
              <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-primary/10 text-primary text-sm font-medium mb-6">
                <BookOpen className="w-4 h-4" />
                Documentation
              </div>
              <h1 className="text-4xl md:text-5xl font-bold tracking-tight">
                Learn Termy
              </h1>
              <p className="mt-4 text-lg text-muted-foreground max-w-2xl leading-relaxed">
                Everything you need to know about installing, configuring, and 
                mastering your new terminal experience.
              </p>
            </div>

            {/* Start here cards */}
            <div className="mb-14">
              <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-4">
                Start Here
              </h2>
              <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                {START_HERE_DOCS.map((doc) => (
                  <Link
                    key={doc.slug}
                    to="/docs/$"
                    params={{ _splat: doc.slug }}
                    className="group relative p-5 rounded-2xl bg-card/50 border border-border/50 hover:border-primary/30 hover:bg-card/80 transition-all duration-300"
                  >
                    <div className="flex items-start justify-between mb-3">
                      <div className="w-10 h-10 rounded-xl bg-primary/10 flex items-center justify-center text-primary group-hover:scale-110 transition-transform duration-300">
                        <doc.icon className="w-5 h-5" />
                      </div>
                      <ArrowRight className="w-4 h-4 text-muted-foreground opacity-0 -translate-x-2 group-hover:opacity-100 group-hover:translate-x-0 transition-all duration-300" />
                    </div>
                    <h3 className="font-semibold text-foreground group-hover:text-primary transition-colors">
                      {doc.label}
                    </h3>
                  </Link>
                ))}
              </div>
            </div>

            {/* Category sections */}
            <div className="space-y-12">
              {categories.map((category) => (
                <div key={category}>
                  <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-4">
                    {category}
                  </h2>
                  <div className="grid gap-3 sm:grid-cols-2">
                    {docsByCategory[category].map((doc) => (
                      <Link
                        key={doc.slug}
                        to="/docs/$"
                        params={{ _splat: doc.slug }}
                        className="group flex flex-col p-4 rounded-xl border border-border/30 bg-secondary/20 hover:border-primary/20 hover:bg-secondary/40 transition-all duration-200"
                      >
                        <h3 className="font-medium text-foreground group-hover:text-primary transition-colors">
                          {doc.title}
                        </h3>
                        {doc.description && (
                          <p className="mt-1.5 text-sm text-muted-foreground line-clamp-2">
                            {doc.description}
                          </p>
                        )}
                      </Link>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </main>

          {/* Right spacing for xl screens */}
          <div className="hidden xl:block w-56 shrink-0" />
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
