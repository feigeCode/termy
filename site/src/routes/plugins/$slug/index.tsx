import { Link, createFileRoute } from "@tanstack/react-router";
import type { JSX } from "react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  type RegistryPlugin,
  type RegistryPluginVersion,
  fetchPluginWithVersions,
} from "@/lib/plugin-registry";

export const Route = createFileRoute("/plugins/$slug/")({
  component: PluginDetailPage,
});

function PluginDetailPage(): JSX.Element {
  const { slug } = Route.useParams();
  const [plugin, setPlugin] = useState<RegistryPlugin | null>(null);
  const [versions, setVersions] = useState<RegistryPluginVersion[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const latestVersion = versions[0] ?? null;

  const installSnippet = useMemo(
    () => `mkdir -p ~/.config/termy/plugins/${slug}`,
    [slug],
  );

  useEffect(() => {
    void load();
  }, [slug]);

  async function load(): Promise<void> {
    try {
      setLoading(true);
      setError(null);
      const response = await fetchPluginWithVersions(slug);
      setPlugin(response.plugin);
      setVersions(response.versions);
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return (
      <section className="pt-28 pb-16">
        <div className="mx-auto max-w-6xl rounded-xl border border-border/60 bg-card/50 px-4 py-6 text-center text-sm text-muted-foreground">
          Loading plugin...
        </div>
      </section>
    );
  }

  if (error || !plugin) {
    return (
      <section className="pt-28 pb-16">
        <div className="mx-auto max-w-6xl space-y-4">
          <Button asChild variant="outline">
            <Link to="/plugins">Back to plugins</Link>
          </Button>
          <div className="rounded-xl border border-destructive/40 bg-destructive/10 px-4 py-3 text-sm text-destructive">
            {error ?? "Plugin not found"}
          </div>
        </div>
      </section>
    );
  }

  return (
    <section className="pt-28 pb-16">
      <div className="mx-auto max-w-6xl space-y-8">
        <div className="flex flex-wrap items-center gap-3">
          <Button asChild variant="outline">
            <Link to="/plugins">Back to plugins</Link>
          </Button>
        </div>

        <div className="mx-auto max-w-3xl px-6 text-center">
          <h1
            className="text-4xl font-bold tracking-tight md:text-6xl animate-blur-in"
            style={{ animationDelay: "0ms" }}
          >
            <span className="gradient-text">{plugin.name}</span>
          </h1>
          <p
            className="mt-4 text-lg text-muted-foreground animate-blur-in"
            style={{ animationDelay: "100ms" }}
          >
            {plugin.description || "No description provided."}
          </p>
          <div
            className="mt-4 flex flex-wrap items-center justify-center gap-3 text-sm text-muted-foreground animate-blur-in"
            style={{ animationDelay: "180ms" }}
          >
            <span className="font-mono text-primary/60">{plugin.slug}</span>
            {plugin.authorName && <span>by {plugin.authorName}</span>}
            {plugin.latestVersion && (
              <span className="rounded bg-primary/10 px-2 py-0.5 text-xs text-primary">
                Latest {plugin.latestVersion}
              </span>
            )}
            {plugin.license && <span>{plugin.license}</span>}
          </div>
          <div
            className="mt-6 flex flex-wrap items-center justify-center gap-3 animate-blur-in"
            style={{ animationDelay: "240ms" }}
          >
            {plugin.repositoryUrl && (
              <Button asChild>
                <a href={plugin.repositoryUrl} target="_blank" rel="noreferrer">
                  Repository
                </a>
              </Button>
            )}
            {plugin.homepageUrl && (
              <Button asChild variant="outline">
                <a href={plugin.homepageUrl} target="_blank" rel="noreferrer">
                  Homepage
                </a>
              </Button>
            )}
            {plugin.latestManifestUrl && (
              <Button asChild variant="outline">
                <a href={plugin.latestManifestUrl} target="_blank" rel="noreferrer">
                  Manifest
                </a>
              </Button>
            )}
            {plugin.latestArtifactUrl && (
              <Button asChild variant="outline">
                <a href={plugin.latestArtifactUrl} target="_blank" rel="noreferrer">
                  Download artifact
                </a>
              </Button>
            )}
          </div>
        </div>

        <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
          <div className="space-y-6">
            <PanelSection title="Registry Snapshot">
              <div className="grid gap-4 md:grid-cols-3">
                <BadgePanel
                  title="Capabilities"
                  values={plugin.latestCapabilities}
                />
                <BadgePanel
                  title="Permissions"
                  values={plugin.latestPermissions}
                />
                <BadgePanel
                  title="Subscriptions"
                  values={plugin.latestSubscriptions}
                />
              </div>
            </PanelSection>

            <PanelSection title="Version History">
              <div className="space-y-3">
                {versions.map((version) => (
                  <div
                    key={version.id}
                    className="rounded-xl border border-border/60 bg-background/60 px-4 py-4"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div>
                        <div className="font-medium text-foreground">
                          {version.version}
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {new Date(version.publishedAt).toLocaleString()}
                        </div>
                      </div>
                      <div className="flex flex-wrap gap-2">
                        {version.manifestUrl && (
                          <a
                            href={version.manifestUrl}
                            target="_blank"
                            rel="noreferrer"
                            className="text-xs text-primary hover:underline"
                          >
                            Manifest
                          </a>
                        )}
                        {version.artifactUrl && (
                          <a
                            href={version.artifactUrl}
                            target="_blank"
                            rel="noreferrer"
                            className="text-xs text-primary hover:underline"
                          >
                            Artifact
                          </a>
                        )}
                      </div>
                    </div>
                    {version.summary && (
                      <p className="mt-3 text-sm leading-relaxed text-muted-foreground">
                        {version.summary}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            </PanelSection>

            {latestVersion?.readme && (
              <PanelSection title="Latest README">
                <div className="whitespace-pre-wrap text-sm leading-7 text-muted-foreground">
                  {latestVersion.readme}
                </div>
              </PanelSection>
            )}
          </div>

          <div className="space-y-6">
            <PanelSection title="Install Foundation">
              <p className="text-sm leading-relaxed text-muted-foreground">
                Full registry installs inside Termy are still in progress. This page
                gives the metadata and artifact hooks the desktop flow will use.
              </p>
              <div className="mt-4 rounded-xl border border-border/60 bg-background/70 p-4">
                <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/60">
                  Local folder seed
                </div>
                <code className="mt-2 block whitespace-pre-wrap text-xs text-foreground">
                  {installSnippet}
                </code>
              </div>
            </PanelSection>

            <PanelSection title="Latest Metadata">
              {latestVersion ? (
                <div className="space-y-4">
                  <BadgePanel title="Capabilities" values={latestVersion.capabilities} />
                  <BadgePanel title="Permissions" values={latestVersion.permissions} />
                  <BadgePanel title="Subscriptions" values={latestVersion.subscriptions} />
                </div>
              ) : (
                <div className="text-sm text-muted-foreground">
                  No published versions yet.
                </div>
              )}
            </PanelSection>
          </div>
        </div>
      </div>
    </section>
  );
}

function PanelSection({
  title,
  children,
}: {
  title: string;
  children: JSX.Element | JSX.Element[] | string;
}): JSX.Element {
  return (
    <div className="rounded-2xl border border-border/40 bg-card/30 p-5 sm:p-6">
      <h2 className="text-lg font-semibold text-foreground">{title}</h2>
      <div className="mt-4">{children}</div>
    </div>
  );
}

function BadgePanel({
  title,
  values,
}: {
  title: string;
  values: string[];
}): JSX.Element {
  return (
    <div>
      <div className="mb-2 text-[11px] font-medium uppercase tracking-[0.18em] text-muted-foreground/60">
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
          <span className="text-sm text-muted-foreground">None</span>
        )}
      </div>
    </div>
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
