CREATE TABLE plugin (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    latest_version TEXT,
    repository_url TEXT,
    homepage_url TEXT,
    license TEXT,
    author_name TEXT,
    is_public BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE plugin_version (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plugin_id UUID NOT NULL REFERENCES plugin(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    readme TEXT NOT NULL DEFAULT '',
    manifest_url TEXT,
    artifact_url TEXT,
    checksum_sha256 TEXT,
    permissions TEXT[] NOT NULL DEFAULT '{}',
    capabilities TEXT[] NOT NULL DEFAULT '{}',
    subscriptions TEXT[] NOT NULL DEFAULT '{}',
    created_by TEXT,
    published_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(plugin_id, version)
);

CREATE INDEX plugin_slug_idx ON plugin(slug);
CREATE INDEX plugin_version_plugin_id_idx ON plugin_version(plugin_id);
CREATE INDEX plugin_version_plugin_id_published_at_idx
    ON plugin_version(plugin_id, published_at DESC);
