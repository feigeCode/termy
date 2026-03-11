export interface RegistryPlugin {
  id: string;
  name: string;
  slug: string;
  description: string;
  latestVersion: string | null;
  repositoryUrl: string | null;
  homepageUrl: string | null;
  license: string | null;
  authorName: string | null;
  githubUsernameClaim: string;
  githubUserIdClaim: number | null;
  isPublic: boolean;
  latestCapabilities: string[];
  latestPermissions: string[];
  latestSubscriptions: string[];
  latestManifestUrl: string | null;
  latestArtifactUrl: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface RegistryPluginVersion {
  id: string;
  pluginId: string;
  version: string;
  summary: string;
  readme: string;
  manifestUrl: string | null;
  artifactUrl: string | null;
  checksumSha256: string | null;
  permissions: string[];
  capabilities: string[];
  subscriptions: string[];
  createdBy: string | null;
  publishedAt: string;
  createdAt: string;
}

export interface RegistryPluginWithVersionsResponse {
  plugin: RegistryPlugin;
  versions: RegistryPluginVersion[];
}

interface ApiErrorBody {
  error?: string;
}

const API_BASE = "/theme-api";

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers ?? {});
  if (init?.body && !(init.body instanceof FormData)) {
    headers.set("Content-Type", "application/json");
  }

  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    credentials: "include",
    headers,
  });

  if (!response.ok) {
    let message = `Request failed (${response.status})`;
    try {
      const body = (await response.json()) as ApiErrorBody;
      if (body.error) {
        message = body.error;
      }
    } catch {
      // keep default message
    }
    throw new Error(message);
  }

  return (await response.json()) as T;
}

export async function fetchPlugins(): Promise<RegistryPlugin[]> {
  return requestJson<RegistryPlugin[]>("/plugins");
}

export async function fetchMyPlugins(): Promise<RegistryPlugin[]> {
  return requestJson<RegistryPlugin[]>("/plugins/me");
}

export async function fetchPluginWithVersions(
  slug: string,
): Promise<RegistryPluginWithVersionsResponse> {
  return requestJson<RegistryPluginWithVersionsResponse>(`/plugins/${slug}/versions`);
}

export async function createPlugin(input: {
  name: string;
  description: string;
  isPublic: boolean;
  version: string;
  summary: string;
  readme: string;
  repositoryUrl: string;
  homepageUrl: string;
  license: string;
  authorName: string;
  manifestUrl: string;
  artifactUrl: string;
  checksumSha256: string;
  permissions: string[];
  capabilities: string[];
  subscriptions: string[];
}): Promise<RegistryPlugin> {
  return requestJson<RegistryPlugin>("/plugins", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export async function updatePlugin(
  slug: string,
  input: {
    name: string;
    description: string;
    isPublic: boolean;
    repositoryUrl: string;
    homepageUrl: string;
    license: string;
    authorName: string;
  },
): Promise<RegistryPlugin> {
  return requestJson<RegistryPlugin>(`/plugins/${slug}`, {
    method: "PATCH",
    body: JSON.stringify(input),
  });
}

export async function publishPluginVersion(
  slug: string,
  input: {
    version: string;
    summary: string;
    readme: string;
    manifestUrl: string;
    artifactUrl: string;
    checksumSha256: string;
    permissions: string[];
    capabilities: string[];
    subscriptions: string[];
  },
): Promise<RegistryPluginWithVersionsResponse> {
  return requestJson<RegistryPluginWithVersionsResponse>(`/plugins/${slug}/versions`, {
    method: "POST",
    body: JSON.stringify(input),
  });
}
