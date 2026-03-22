import { useState } from "react";
import { Download, ArrowLeft, Calendar, Zap } from "lucide-react";
import type { ThemeWithVersionsResponse } from "../../lib/theme-store";

interface Props {
  initialData: ThemeWithVersionsResponse;
  slug: string;
}

export default function ThemeDetail({ initialData }: Props) {
  const { theme, versions } = initialData;

  return (
    <div className="mx-auto max-w-3xl px-4 sm:px-6 py-12">
      <a
        href="/themes"
        className="inline-flex items-center gap-1.5 mb-8 text-sm text-muted-foreground transition-colors hover:text-foreground"
      >
        <ArrowLeft className="h-4 w-4" />
        All themes
      </a>

      <div className="mb-8">
        <div className="flex items-start justify-between gap-4">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">{theme.name}</h1>
            <p className="mt-2 text-muted-foreground">{theme.description}</p>
          </div>
          {theme.latestVersion && (
            <span className="shrink-0 font-mono text-sm text-muted-foreground/60 border border-border/40 rounded px-2 py-1">
              v{theme.latestVersion}
            </span>
          )}
        </div>
        <div className="mt-3 flex items-center gap-4 text-xs text-muted-foreground/60 font-mono">
          <span>@{theme.githubUsernameClaim}</span>
          <span>{new Date(theme.createdAt).toLocaleDateString()}</span>
        </div>
      </div>

      <div className="flex flex-wrap gap-3 mb-10">
        <a
          href={`termy://store/theme-install?slug=${encodeURIComponent(theme.slug)}`}
          className="inline-flex items-center gap-2 rounded bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
        >
          <Zap className="h-4 w-4" />
          Install in Termy
        </a>
        {theme.fileUrl && (
          <a
            href={theme.fileUrl}
            download
            className="inline-flex items-center gap-2 rounded border border-border px-4 py-2.5 text-sm text-muted-foreground transition-colors hover:border-border/70 hover:text-foreground"
          >
            <Download className="h-4 w-4" />
            Download file
          </a>
        )}
      </div>

      {versions.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold mb-4">Version history</h2>
          <div className="space-y-3">
            {versions.map((v) => (
              <div key={v.id} className="rounded border border-border/50 p-4">
                <div className="flex items-center justify-between gap-3 mb-2">
                  <span className="font-mono text-sm font-medium text-foreground">v{v.version}</span>
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground/60">
                    <Calendar className="h-3 w-3" />
                    {new Date(v.publishedAt).toLocaleDateString()}
                  </div>
                </div>
                {v.changelog && (
                  <p className="text-sm text-muted-foreground/70">{v.changelog}</p>
                )}
                {v.fileUrl && (
                  <a
                    href={v.fileUrl}
                    download
                    className="mt-3 inline-flex items-center gap-1.5 text-xs text-muted-foreground/60 transition-colors hover:text-primary"
                  >
                    <Download className="h-3 w-3" />
                    Download v{v.version}
                  </a>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
