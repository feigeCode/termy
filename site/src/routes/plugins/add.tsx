import { Link, createFileRoute } from "@tanstack/react-router";
import type { FormEvent, JSX } from "react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  createPlugin,
  fetchMyPlugins,
  fetchPluginWithVersions,
  publishPluginVersion,
  type RegistryPlugin,
  type RegistryPluginVersion,
  updatePlugin,
} from "@/lib/plugin-registry";
import { fetchCurrentUser, getThemeLoginUrl, logout, type AuthUser } from "@/lib/theme-store";

export const Route = createFileRoute("/plugins/add")({
  component: PluginAddPage,
});

export function PluginAddPage(): JSX.Element {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [plugins, setPlugins] = useState<RegistryPlugin[]>([]);
  const [selectedSlug, setSelectedSlug] = useState("");
  const [selectedVersions, setSelectedVersions] = useState<RegistryPluginVersion[]>([]);

  const [createName, setCreateName] = useState("");
  const [createDescription, setCreateDescription] = useState("");
  const [createIsPublic, setCreateIsPublic] = useState(true);
  const [createVersion, setCreateVersion] = useState("1.0.0");
  const [createSummary, setCreateSummary] = useState("");
  const [createReadme, setCreateReadme] = useState("");
  const [createRepositoryUrl, setCreateRepositoryUrl] = useState("");
  const [createHomepageUrl, setCreateHomepageUrl] = useState("");
  const [createLicense, setCreateLicense] = useState("");
  const [createAuthorName, setCreateAuthorName] = useState("");
  const [createManifestUrl, setCreateManifestUrl] = useState("");
  const [createArtifactUrl, setCreateArtifactUrl] = useState("");
  const [createChecksum, setCreateChecksum] = useState("");
  const [createPermissions, setCreatePermissions] = useState("");
  const [createCapabilities, setCreateCapabilities] = useState("");
  const [createSubscriptions, setCreateSubscriptions] = useState("");

  const [updateName, setUpdateName] = useState("");
  const [updateDescription, setUpdateDescription] = useState("");
  const [updateIsPublic, setUpdateIsPublic] = useState(true);
  const [updateRepositoryUrl, setUpdateRepositoryUrl] = useState("");
  const [updateHomepageUrl, setUpdateHomepageUrl] = useState("");
  const [updateLicense, setUpdateLicense] = useState("");
  const [updateAuthorName, setUpdateAuthorName] = useState("");

  const [publishVersion, setPublishVersion] = useState("");
  const [publishSummary, setPublishSummary] = useState("");
  const [publishReadme, setPublishReadme] = useState("");
  const [publishManifestUrl, setPublishManifestUrl] = useState("");
  const [publishArtifactUrl, setPublishArtifactUrl] = useState("");
  const [publishChecksum, setPublishChecksum] = useState("");
  const [publishPermissions, setPublishPermissions] = useState("");
  const [publishCapabilities, setPublishCapabilities] = useState("");
  const [publishSubscriptions, setPublishSubscriptions] = useState("");

  const [isBootstrapping, setIsBootstrapping] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const selectedPlugin = plugins.find((plugin) => plugin.slug === selectedSlug) ?? null;
  const canEditSelectedPlugin =
    Boolean(user) &&
    Boolean(selectedPlugin) &&
    (selectedPlugin?.githubUserIdClaim != null
      ? selectedPlugin.githubUserIdClaim === user?.githubUserId
      : selectedPlugin?.githubUsernameClaim.toLowerCase() ===
        user?.githubLogin.toLowerCase());

  const loginUrl = useMemo(() => getThemeLoginUrl("/plugins/add"), []);

  useEffect(() => {
    void bootstrap();
  }, []);

  useEffect(() => {
    if (!selectedPlugin) {
      return;
    }

    setUpdateName(selectedPlugin.name);
    setUpdateDescription(selectedPlugin.description);
    setUpdateIsPublic(selectedPlugin.isPublic);
    setUpdateRepositoryUrl(selectedPlugin.repositoryUrl ?? "");
    setUpdateHomepageUrl(selectedPlugin.homepageUrl ?? "");
    setUpdateLicense(selectedPlugin.license ?? "");
    setUpdateAuthorName(selectedPlugin.authorName ?? "");
  }, [selectedPlugin]);

  async function bootstrap(): Promise<void> {
    try {
      setError(null);
      setIsBootstrapping(true);
      const currentUser = await fetchCurrentUser();
      setUser(currentUser);

      if (!currentUser) {
        setPlugins([]);
        setSelectedSlug("");
        setSelectedVersions([]);
        return;
      }

      const loadedPlugins = await fetchMyPlugins();
      setPlugins(loadedPlugins);

      if (loadedPlugins.length > 0) {
        const firstSlug = loadedPlugins[0].slug;
        setSelectedSlug(firstSlug);
        await loadPluginVersions(firstSlug);
      }
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setIsBootstrapping(false);
    }
  }

  async function loadPluginVersions(slug: string): Promise<void> {
    try {
      const response = await fetchPluginWithVersions(slug);
      setSelectedVersions(response.versions);
    } catch (err) {
      setSelectedVersions([]);
      setError(getErrorMessage(err));
    }
  }

  async function handleLogout(): Promise<void> {
    try {
      setError(null);
      await logout();
      setUser(null);
      setNotice("Logged out.");
    } catch (err) {
      setError(getErrorMessage(err));
    }
  }

  async function handleCreatePlugin(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();

    try {
      setError(null);
      setNotice(null);
      setIsSubmitting(true);

      const created = await createPlugin({
        name: createName,
        description: createDescription,
        isPublic: createIsPublic,
        version: createVersion,
        summary: createSummary,
        readme: createReadme,
        repositoryUrl: createRepositoryUrl,
        homepageUrl: createHomepageUrl,
        license: createLicense,
        authorName: createAuthorName,
        manifestUrl: createManifestUrl,
        artifactUrl: createArtifactUrl,
        checksumSha256: createChecksum,
        permissions: parseListInput(createPermissions),
        capabilities: parseListInput(createCapabilities),
        subscriptions: parseListInput(createSubscriptions),
      });

      setPlugins((prev) => [created, ...prev.filter((item) => item.id !== created.id)]);
      setSelectedSlug(created.slug);
      setSelectedVersions([]);
      setCreateName("");
      setCreateDescription("");
      setCreateVersion("1.0.0");
      setCreateSummary("");
      setCreateReadme("");
      setCreateRepositoryUrl("");
      setCreateHomepageUrl("");
      setCreateLicense("");
      setCreateAuthorName("");
      setCreateManifestUrl("");
      setCreateArtifactUrl("");
      setCreateChecksum("");
      setCreatePermissions("");
      setCreateCapabilities("");
      setCreateSubscriptions("");
      setNotice(`Plugin '${created.slug}' created.`);
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handleUpdatePlugin(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    if (!selectedPlugin) {
      return;
    }

    try {
      setError(null);
      setNotice(null);
      setIsSubmitting(true);

      const updated = await updatePlugin(selectedPlugin.slug, {
        name: updateName,
        description: updateDescription,
        isPublic: updateIsPublic,
        repositoryUrl: updateRepositoryUrl,
        homepageUrl: updateHomepageUrl,
        license: updateLicense,
        authorName: updateAuthorName,
      });

      setPlugins((prev) => prev.map((item) => (item.id === updated.id ? updated : item)));
      setNotice(`Plugin '${updated.slug}' updated.`);
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handlePublishVersion(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    if (!selectedPlugin) {
      return;
    }

    try {
      setError(null);
      setNotice(null);
      setIsSubmitting(true);

      const response = await publishPluginVersion(selectedPlugin.slug, {
        version: publishVersion,
        summary: publishSummary,
        readme: publishReadme,
        manifestUrl: publishManifestUrl,
        artifactUrl: publishArtifactUrl,
        checksumSha256: publishChecksum,
        permissions: parseListInput(publishPermissions),
        capabilities: parseListInput(publishCapabilities),
        subscriptions: parseListInput(publishSubscriptions),
      });

      setPlugins((prev) =>
        prev.map((item) => (item.id === response.plugin.id ? response.plugin : item)),
      );
      setSelectedVersions(response.versions);
      setPublishVersion("");
      setPublishSummary("");
      setPublishReadme("");
      setPublishManifestUrl("");
      setPublishArtifactUrl("");
      setPublishChecksum("");
      setPublishPermissions("");
      setPublishCapabilities("");
      setPublishSubscriptions("");
      setNotice("Plugin version published.");
    } catch (err) {
      setError(getErrorMessage(err));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handleSelectPlugin(slug: string): Promise<void> {
    setSelectedSlug(slug);
    setNotice(null);
    setError(null);
    await loadPluginVersions(slug);
  }

  return (
    <section className="pt-28 pb-16">
      <div className="mx-auto max-w-6xl space-y-6">
        <div className="mx-auto max-w-3xl px-6 text-center">
          <h1 className="text-4xl font-bold tracking-tight md:text-6xl animate-blur-in">
            <span className="gradient-text">plugin releases.</span>
          </h1>
          <p className="mt-4 text-lg text-muted-foreground animate-blur-in">
            Publish plugin metadata now, then connect artifacts and native install
            flows as the registry grows.
          </p>
          <div className="mt-6 flex flex-wrap items-center justify-center gap-3 animate-blur-in">
            <Button asChild variant="outline">
              <Link to="/plugins">Back to registry</Link>
            </Button>
            {user ? (
              <>
                <div className="rounded-lg border border-border/60 bg-background/80 px-3 py-2 text-sm">
                  Signed in as <span className="font-medium">@{user.githubLogin}</span>
                </div>
                <Button type="button" variant="outline" onClick={handleLogout}>
                  Log out
                </Button>
              </>
            ) : (
              <a href={loginUrl}>
                <Button type="button">Login with GitHub</Button>
              </a>
            )}
          </div>
        </div>

        {error && <Notice kind="error" message={error} />}
        {notice && <Notice kind="success" message={notice} />}

        {!isBootstrapping && !user && (
          <Card className="border-border/60">
            <CardHeader>
              <CardTitle>Authentication Required</CardTitle>
              <CardDescription>
                Sign in with GitHub to manage plugin registry entries.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <a href={loginUrl}>
                <Button type="button">Login with GitHub</Button>
              </a>
            </CardContent>
          </Card>
        )}

        {user && (
          <div className="grid gap-6 lg:grid-cols-[360px_minmax(0,1fr)] animate-blur-in">
            <Card className="border-border/60">
              <CardHeader>
                <CardTitle>Your Plugins</CardTitle>
                <CardDescription>
                  {isBootstrapping ? "Loading plugins..." : `${plugins.length} plugins available`}
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-2">
                {plugins.map((plugin) => (
                  <button
                    key={plugin.id}
                    type="button"
                    className={`w-full rounded-lg border px-3 py-2 text-left transition-colors ${
                      selectedSlug === plugin.slug
                        ? "border-primary/60 bg-primary/10"
                        : "border-border/50 bg-background hover:border-primary/30"
                    }`}
                    onClick={() => void handleSelectPlugin(plugin.slug)}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <p className="font-medium">{plugin.name}</p>
                      <span className="text-xs text-muted-foreground">
                        {plugin.latestVersion ?? "no versions"}
                      </span>
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">/{plugin.slug}</p>
                  </button>
                ))}
                {plugins.length === 0 && !isBootstrapping && (
                  <p className="text-sm text-muted-foreground">No plugins created yet.</p>
                )}
              </CardContent>
            </Card>

            <div className="space-y-6">
              <Card className="border-border/60">
                <CardHeader>
                  <CardTitle>Create Plugin</CardTitle>
                  <CardDescription>
                    Create a new plugin registry entry and publish the initial version.
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  <form className="space-y-3" onSubmit={(event) => void handleCreatePlugin(event)}>
                    <input className={fieldClass} placeholder="Plugin name" value={createName} onChange={(event) => setCreateName(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Initial version (e.g. 1.0.0)" value={createVersion} onChange={(event) => setCreateVersion(event.target.value)} disabled={!user || isSubmitting} />
                    <textarea className={areaClass} placeholder="Description" value={createDescription} onChange={(event) => setCreateDescription(event.target.value)} disabled={!user || isSubmitting} />
                    <label className="flex items-center gap-2 text-sm text-muted-foreground">
                      <input type="checkbox" checked={createIsPublic} onChange={(event) => setCreateIsPublic(event.target.checked)} disabled={!user || isSubmitting} />
                      Public plugin
                    </label>
                    <input className={fieldClass} placeholder="Repository URL" value={createRepositoryUrl} onChange={(event) => setCreateRepositoryUrl(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Homepage URL" value={createHomepageUrl} onChange={(event) => setCreateHomepageUrl(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="License" value={createLicense} onChange={(event) => setCreateLicense(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Author name" value={createAuthorName} onChange={(event) => setCreateAuthorName(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Manifest URL" value={createManifestUrl} onChange={(event) => setCreateManifestUrl(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Artifact URL" value={createArtifactUrl} onChange={(event) => setCreateArtifactUrl(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Checksum SHA256" value={createChecksum} onChange={(event) => setCreateChecksum(event.target.value)} disabled={!user || isSubmitting} />
                    <textarea className={areaClass} placeholder="Version summary" value={createSummary} onChange={(event) => setCreateSummary(event.target.value)} disabled={!user || isSubmitting} />
                    <textarea className="min-h-36 w-full rounded-lg border border-border bg-background px-3 py-2 font-mono text-sm" placeholder="README / release notes" value={createReadme} onChange={(event) => setCreateReadme(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Permissions (comma or newline separated)" value={createPermissions} onChange={(event) => setCreatePermissions(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Capabilities (comma or newline separated)" value={createCapabilities} onChange={(event) => setCreateCapabilities(event.target.value)} disabled={!user || isSubmitting} />
                    <input className={fieldClass} placeholder="Subscriptions (comma or newline separated)" value={createSubscriptions} onChange={(event) => setCreateSubscriptions(event.target.value)} disabled={!user || isSubmitting} />
                    <Button type="submit" disabled={!user || isSubmitting}>Create plugin</Button>
                  </form>
                </CardContent>
              </Card>

              <Card className="border-border/60">
                <CardHeader>
                  <CardTitle>Selected Plugin</CardTitle>
                  <CardDescription>
                    {selectedPlugin ? `${selectedPlugin.name} (${selectedPlugin.slug})` : "Select a plugin from the list"}
                  </CardDescription>
                </CardHeader>
                <CardContent>
                  {selectedPlugin && (
                    <div className="space-y-6">
                      <form className="space-y-3" onSubmit={(event) => void handleUpdatePlugin(event)}>
                        <h3 className="text-sm font-semibold">Update metadata</h3>
                        <input className={fieldClass} value={updateName} onChange={(event) => setUpdateName(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <textarea className={areaClass} value={updateDescription} onChange={(event) => setUpdateDescription(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <label className="flex items-center gap-2 text-sm text-muted-foreground">
                          <input type="checkbox" checked={updateIsPublic} onChange={(event) => setUpdateIsPublic(event.target.checked)} disabled={!canEditSelectedPlugin || isSubmitting} />
                          Public plugin
                        </label>
                        <input className={fieldClass} placeholder="Repository URL" value={updateRepositoryUrl} onChange={(event) => setUpdateRepositoryUrl(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Homepage URL" value={updateHomepageUrl} onChange={(event) => setUpdateHomepageUrl(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="License" value={updateLicense} onChange={(event) => setUpdateLicense(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Author name" value={updateAuthorName} onChange={(event) => setUpdateAuthorName(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <Button type="submit" disabled={!canEditSelectedPlugin || isSubmitting}>Save changes</Button>
                      </form>

                      <form className="space-y-3" onSubmit={(event) => void handlePublishVersion(event)}>
                        <h3 className="text-sm font-semibold">Publish new version</h3>
                        <input className={fieldClass} placeholder="Version (e.g. 1.2.0)" value={publishVersion} onChange={(event) => setPublishVersion(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <textarea className={areaClass} placeholder="Version summary" value={publishSummary} onChange={(event) => setPublishSummary(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <textarea className="min-h-36 w-full rounded-lg border border-border bg-background px-3 py-2 font-mono text-sm" placeholder="README / release notes" value={publishReadme} onChange={(event) => setPublishReadme(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Manifest URL" value={publishManifestUrl} onChange={(event) => setPublishManifestUrl(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Artifact URL" value={publishArtifactUrl} onChange={(event) => setPublishArtifactUrl(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Checksum SHA256" value={publishChecksum} onChange={(event) => setPublishChecksum(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Permissions (comma or newline separated)" value={publishPermissions} onChange={(event) => setPublishPermissions(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Capabilities (comma or newline separated)" value={publishCapabilities} onChange={(event) => setPublishCapabilities(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <input className={fieldClass} placeholder="Subscriptions (comma or newline separated)" value={publishSubscriptions} onChange={(event) => setPublishSubscriptions(event.target.value)} disabled={!canEditSelectedPlugin || isSubmitting} />
                        <Button type="submit" disabled={!canEditSelectedPlugin || isSubmitting}>Publish version</Button>
                      </form>

                      <div>
                        <h3 className="mb-2 text-sm font-semibold">Version history</h3>
                        <div className="space-y-2">
                          {selectedVersions.map((version) => (
                            <div key={version.id} className="rounded-lg border border-border/60 px-3 py-2">
                              <div className="flex items-center justify-between gap-3">
                                <span className="font-medium">{version.version}</span>
                                <span className="text-xs text-muted-foreground">{new Date(version.publishedAt).toLocaleString()}</span>
                              </div>
                              {version.summary && <p className="mt-2 text-sm text-muted-foreground">{version.summary}</p>}
                            </div>
                          ))}
                          {selectedVersions.length === 0 && (
                            <p className="text-sm text-muted-foreground">No versions published yet.</p>
                          )}
                        </div>
                      </div>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

const fieldClass =
  "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm";

const areaClass =
  "min-h-20 w-full rounded-lg border border-border bg-background px-3 py-2 text-sm";

function Notice({
  kind,
  message,
}: {
  kind: "error" | "success";
  message: string;
}): JSX.Element {
  return (
    <div
      className={`rounded-xl px-4 py-3 text-sm ${
        kind === "error"
          ? "border border-destructive/40 bg-destructive/10 text-destructive"
          : "border border-primary/40 bg-primary/10 text-foreground"
      }`}
    >
      {message}
    </div>
  );
}

function parseListInput(value: string): string[] {
  return value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean);
}

function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }

  return "Unexpected error";
}
