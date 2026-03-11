import { Link, createFileRoute } from "@tanstack/react-router";
import type { JSX } from "react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  type RegistryPlugin,
  fetchPlugins,
} from "@/lib/plugin-registry";

export const Route = createFileRoute("/plugins/")({
  component: PluginRegistryPage,
});

function PluginRegistryPage(): JSX.Element {
  const [plugins, setPlugins] = useState<RegistryPlugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const filteredPlugins = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();

    if (!query) {
      return plugins;
    }

    return plugins.filter((plugin) =>
      [
        plugin.name,
        plugin.slug,
        plugin.description,
        plugin.authorName ?? "",
        plugin.latestCapabilities.join(" "),
        plugin.latestPermissions.join(" "),
        plugin.latestSubscriptions.join(" "),
      ]
        .join(" ")
        .toLowerCase()
        .includes(query),
    );
  }, [plugins, searchQuery]);

  useEffect(() => {
    void load();
  }, []);

  async function load(): Promise<void> {
    try {
      setLoading(true);
      setError(null);
      setPlugins(await fetchPlugins());
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <section className="pt-28 pb-16">
      <div className="mx-auto max-w-6xl space-y-10">
        <div className="mx-auto max-w-3xl px-6 text-center">
          <h1
            className="text-4xl font-bold tracking-tight md:text-6xl animate-blur-in"
            style={{ animationDelay: "0ms" }}
          >
            <span className="gradient-text">plugins.</span>
          </h1>
          <p
            className="mt-4 text-lg text-muted-foreground animate-blur-in"
            style={{ animationDelay: "100ms" }}
          >
            Browse the early Termy plugin registry foundation and inspect what each
            plugin exposes before install flows land.
          </p>
          <div
            className="mt-6 flex flex-wrap items-center justify-center gap-3 animate-blur-in"
            style={{ animationDelay: "200ms" }}
          >
            <Button asChild>
              <Link to="/plugins/add">Publish a plugin</Link>
            </Button>
            <Button asChild>
              <Link to="/docs">Read plugin docs</Link>
            </Button>
            <Button asChild variant="outline">
              <a
                href="https://github.com/lassejlv/termy/tree/main/crates/plugin_example_status"
                target="_blank"
                rel="noreferrer"
              >
                View Rust example
              </a>
            </Button>
          </div>
        </div>

        {error && (
          <div className="rounded-xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error}
          </div>
        )}

        <div className="mx-auto w-full max-w-2xl px-6">
          <div className="rounded-2xl border border-border/60 bg-card/40 p-2 backdrop-blur-sm">
            <input
              type="search"
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="Search plugins by name, slug, permissions, capabilities, or author..."
              className="w-full rounded-xl border border-transparent bg-background/70 px-4 py-3 text-sm text-foreground outline-none transition-colors placeholder:text-muted-foreground/70 focus:border-primary/40"
              aria-label="Search plugins"
            />
          </div>
        </div>

        <div className="grid gap-4 lg:grid-cols-2">
          {filteredPlugins.map((plugin, index) => (
            <article
              key={plugin.id}
              className="animate-blur-in rounded-2xl border border-border/40 bg-card/30 p-5 transition-all duration-300 hover:border-primary/25 hover:bg-card/55"
              style={{ animationDelay: `${(index + 1) * 80}ms` }}
            >
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="space-y-1">
                  <Link
                    to="/plugins/$slug"
                    params={{ slug: plugin.slug }}
                    className="text-lg font-semibold text-foreground hover:text-primary"
                  >
                    {plugin.name}
                  </Link>
                  <div className="text-xs font-mono text-primary/60">{plugin.slug}</div>
                </div>
                {plugin.latestVersion && (
                  <span className="rounded bg-primary/10 px-2 py-1 text-xs text-primary">
                    {plugin.latestVersion}
                  </span>
                )}
              </div>

              <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                {plugin.description || "No description provided."}
              </p>

              <div className="mt-4 flex flex-wrap gap-2 text-xs text-muted-foreground">
                {plugin.authorName && <MetaPill label={`by ${plugin.authorName}`} />}
                {plugin.license && <MetaPill label={plugin.license} />}
                <MetaPill
                  label={`${plugin.latestCapabilities.length} capabilities`}
                />
                <MetaPill
                  label={`${plugin.latestPermissions.length} permissions`}
                />
                <MetaPill
                  label={`${plugin.latestSubscriptions.length} subscriptions`}
                />
              </div>

              <div className="mt-4 space-y-3">
                <BadgeRow title="Capabilities" values={plugin.latestCapabilities} />
                <BadgeRow title="Permissions" values={plugin.latestPermissions} />
                <BadgeRow title="Subscriptions" values={plugin.latestSubscriptions} />
              </div>

              <div className="mt-5 flex flex-wrap gap-3">
                <Button asChild size="sm">
                  <Link to="/plugins/$slug" params={{ slug: plugin.slug }}>
                    Open details
                  </Link>
                </Button>
                {plugin.repositoryUrl && (
                  <Button asChild size="sm" variant="outline">
                    <a href={plugin.repositoryUrl} target="_blank" rel="noreferrer">
                      Repository
                    </a>
                  </Button>
                )}
                {!plugin.repositoryUrl && (
                  <span className="self-center text-xs text-muted-foreground">
                    Registry install flow coming soon
                  </span>
                )}
              </div>
            </article>
          ))}
        </div>

        {!loading && plugins.length === 0 && (
          <div className="rounded-xl border border-border/60 bg-card/50 px-4 py-6 text-center text-sm text-muted-foreground">
            No public plugins are published yet.
          </div>
        )}

        {!loading && plugins.length > 0 && filteredPlugins.length === 0 && (
          <div className="rounded-xl border border-border/60 bg-card/50 px-4 py-6 text-center text-sm text-muted-foreground">
            No plugins match &quot;{searchQuery.trim()}&quot;.
          </div>
        )}

        {loading && (
          <div className="rounded-xl border border-border/60 bg-card/50 px-4 py-6 text-center text-sm text-muted-foreground">
            Loading plugins...
          </div>
        )}
      </div>
    </section>
  );
}

function BadgeRow({
  title,
  values,
}: {
  title: string;
  values: string[];
}): JSX.Element {
  return (
    <div>
      <div className="mb-1 text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground/60">
        {title}
      </div>
      <div className="flex flex-wrap gap-2">
        {values.length > 0 ? (
          values.map((value) => (
            <span
              key={`${title}-${value}`}
              className="rounded-full border border-border/60 bg-background/70 px-2.5 py-1 text-[11px] text-foreground/85"
            >
              {formatRegistryToken(value)}
            </span>
          ))
        ) : (
          <span className="text-xs text-muted-foreground">None</span>
        )}
      </div>
    </div>
  );
}

function MetaPill({ label }: { label: string }): JSX.Element {
  return (
    <span className="rounded-full border border-border/60 bg-background/60 px-2.5 py-1">
      {label}
    </span>
  );
}

function formatRegistryToken(value: string): string {
  return value.replaceAll("_", " ");
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return "Unexpected error";
}
