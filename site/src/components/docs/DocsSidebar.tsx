import { useState } from "react";
import { ChevronRight, FileText } from "lucide-react";

interface DocItem {
  slug: string;
  title: string;
  category?: string;
  order?: number;
}

interface Props {
  docs: DocItem[];
  currentSlug: string;
}

export default function DocsSidebar({ docs, currentSlug }: Props) {
  const [search, setSearch] = useState("");

  const filtered = docs.filter(
    (d) =>
      d.title.toLowerCase().includes(search.toLowerCase()) &&
      d.category?.toLowerCase() !== "plugins"
  );

  // Group by category
  const uncategorized = filtered.filter((d) => !d.category);
  const categories = [...new Set(filtered.map((d) => d.category).filter(Boolean))] as string[];

  const sortByOrder = (a: DocItem, b: DocItem) => (a.order ?? 99) - (b.order ?? 99);

  return (
    <aside className="hidden lg:block w-56 shrink-0 sticky top-[3.5rem] h-[calc(100vh-3.5rem)] overflow-y-auto py-6 pr-4">
      <div className="mb-4">
        <input
          type="text"
          placeholder="Search docs..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full rounded border border-border bg-secondary/50 px-3 py-1.5 text-xs text-foreground placeholder:text-muted-foreground/60 focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>

      <nav className="space-y-5">
        {uncategorized.sort(sortByOrder).length > 0 && (
          <div>
            <ul className="space-y-0.5">
              {uncategorized.sort(sortByOrder).map((doc) => (
                <li key={doc.slug}>
                  <a
                    href={`/docs/${doc.slug}`}
                    className={`flex items-center gap-2 rounded px-2 py-1.5 text-xs transition-colors ${
                      currentSlug === doc.slug
                        ? "bg-primary/10 text-primary font-medium"
                        : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                    }`}
                  >
                    <FileText className="h-3 w-3 shrink-0 opacity-50" />
                    {doc.title}
                  </a>
                </li>
              ))}
            </ul>
          </div>
        )}

        {categories.map((cat) => {
          const catDocs = filtered.filter((d) => d.category === cat).sort(sortByOrder);
          return (
            <div key={cat}>
              <div className="mb-1.5 flex items-center gap-1 px-2">
                <ChevronRight className="h-3 w-3 text-muted-foreground/50" />
                <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/60">{cat}</span>
              </div>
              <ul className="space-y-0.5">
                {catDocs.map((doc) => (
                  <li key={doc.slug}>
                    <a
                      href={`/docs/${doc.slug}`}
                      className={`flex items-center gap-2 rounded px-2 py-1.5 text-xs transition-colors ${
                        currentSlug === doc.slug
                          ? "bg-primary/10 text-primary font-medium"
                          : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                      }`}
                    >
                      <FileText className="h-3 w-3 shrink-0 opacity-50" />
                      {doc.title}
                    </a>
                  </li>
                ))}
              </ul>
            </div>
          );
        })}
      </nav>
    </aside>
  );
}
