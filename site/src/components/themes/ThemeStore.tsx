import { useState, useEffect } from "react";
import { Search, Plus, Download } from "lucide-react";
import type { Theme, AuthUser } from "../../lib/theme-store";
import { fetchCurrentUser, getThemeLoginUrl, logout } from "../../lib/theme-store";

interface Props {
  initialThemes: Theme[];
}

export default function ThemeStore({ initialThemes }: Props) {
  const [themes] = useState<Theme[]>(initialThemes);
  const [search, setSearch] = useState("");
  const [user, setUser] = useState<AuthUser | null>(null);
  const [authLoading, setAuthLoading] = useState(true);

  useEffect(() => {
    fetchCurrentUser()
      .then(setUser)
      .catch(() => setUser(null))
      .finally(() => setAuthLoading(false));
  }, []);

  const filtered = themes.filter(
    (t) =>
      t.isPublic &&
      (t.name.toLowerCase().includes(search.toLowerCase()) ||
        t.description.toLowerCase().includes(search.toLowerCase()))
  );

  return (
    <div className="mx-auto max-w-6xl px-4 sm:px-6 py-12">
      <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">Themes</h1>
          <p className="mt-1.5 text-muted-foreground">Community themes for Termy.</p>
        </div>
        <div className="flex items-center gap-3">
          {!authLoading && (
            user ? (
              <div className="flex items-center gap-3">
                <span className="text-xs text-muted-foreground">@{user.githubLogin}</span>
                <a
                  href="/themes/add"
                  className="inline-flex items-center gap-1.5 rounded border border-border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:border-border/70 hover:text-foreground"
                >
                  <Plus className="h-3.5 w-3.5" />
                  Add theme
                </a>
                <button
                  onClick={() => logout().then(() => setUser(null))}
                  className="text-xs text-muted-foreground/60 transition-colors hover:text-muted-foreground"
                >
                  Sign out
                </button>
              </div>
            ) : (
              <a
                href={getThemeLoginUrl("/themes")}
                className="inline-flex items-center gap-1.5 rounded border border-border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:border-border/70 hover:text-foreground"
              >
                Sign in with GitHub
              </a>
            )
          )}
        </div>
      </div>

      <div className="relative mb-6">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground/50" />
        <input
          type="text"
          placeholder="Search themes..."
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full rounded border border-border bg-card/40 py-2.5 pl-9 pr-4 text-sm text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring"
        />
      </div>

      {filtered.length === 0 ? (
        <p className="text-center py-16 text-muted-foreground text-sm">No themes found.</p>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {filtered.map((theme) => (
            <a
              key={theme.id}
              href={`/themes/${theme.slug}`}
              className="group flex flex-col rounded border border-border/50 p-5 transition-colors hover:border-border hover:bg-card/40"
            >
              <div className="flex items-start justify-between gap-2 mb-2">
                <h3 className="font-semibold text-foreground truncate">{theme.name}</h3>
                {theme.latestVersion && (
                  <span className="shrink-0 font-mono text-[10px] text-muted-foreground/60 border border-border/40 rounded px-1.5 py-0.5">
                    v{theme.latestVersion}
                  </span>
                )}
              </div>
              <p className="text-sm text-muted-foreground/70 leading-relaxed flex-1 line-clamp-2">{theme.description}</p>
              <div className="mt-4 flex items-center justify-between">
                <span className="text-xs text-muted-foreground/50 font-mono">@{theme.githubUsernameClaim}</span>
                {theme.fileUrl && (
                  <span className="flex items-center gap-1 text-xs text-muted-foreground/50 group-hover:text-primary/60 transition-colors">
                    <Download className="h-3 w-3" />
                    Install
                  </span>
                )}
              </div>
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
