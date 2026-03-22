import { useEffect, useState } from "react";
import { Download, ExternalLink } from "lucide-react";
import type { Release, Asset } from "@/lib/types";
import DownloadDialog from "./DownloadDialog";

function getPreferredDownload(assets: Asset[]): Asset | undefined {
  const mac = assets.filter((a) => a.name.toLowerCase().endsWith(".dmg"));
  const windows = assets.filter((a) => {
    const n = a.name.toLowerCase();
    return (
      n.endsWith(".exe") ||
      n.endsWith(".msi") ||
      n.includes("windows") ||
      n.includes("win64") ||
      n.includes("pc-windows")
    );
  });
  const linux = assets.filter((a) => {
    const n = a.name.toLowerCase();
    return (
      (n.includes("linux") && n.endsWith(".tar.gz")) ||
      n.endsWith(".appimage") ||
      n.endsWith(".deb") ||
      n.endsWith(".rpm")
    );
  });

  const agent = navigator.userAgent.toLowerCase();
  if (agent.includes("win")) return windows[0];
  if (agent.includes("mac"))
    return mac.find((a) => a.name.toLowerCase().includes("arm64")) || mac[0];
  if (agent.includes("linux"))
    return (
      linux.find((a) => a.name.toLowerCase().includes("x86_64")) || linux[0]
    );
  return mac.find((a) => a.name.toLowerCase().includes("arm64")) || assets[0];
}

interface Props {
  release: Release | null;
}

export default function DownloadSection({ release: initialRelease }: Props) {
  const [release, setRelease] = useState<Release | null>(initialRelease);
  const [loading, setLoading] = useState(!initialRelease);
  const [dialogOpen, setDialogOpen] = useState(false);

  useEffect(() => {
    if (release) {
      setLoading(false);
      return;
    }

    async function fetchRelease() {
      // Try proxy first, fall back to GitHub API directly
      const urls = [
        "https://api.termy.run/api/github/releases/latest",
        "https://api.github.com/repos/lassejlv/termy/releases/latest",
      ];
      for (const url of urls) {
        try {
          const r = await fetch(url);
          if (r.ok) {
            const data: Release = await r.json();
            setRelease(data);
            return;
          }
        } catch {
          // try next
        }
      }
    }

    fetchRelease().finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="flex flex-wrap gap-3">
        <div className="h-10 w-44 rounded border border-border bg-card/50 animate-pulse" />
        <div className="h-10 w-32 rounded border border-border bg-card/50 animate-pulse" />
      </div>
    );
  }

  if (!release) {
    return (
      <a
        href="https://github.com/lassejlv/termy/releases"
        target="_blank"
        rel="noopener noreferrer"
        className="inline-flex items-center gap-2 rounded bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
      >
        <Download className="h-4 w-4" />
        Download Termy
      </a>
    );
  }

  const preferred = getPreferredDownload(release.assets);

  return (
    <>
      <div className="flex flex-wrap gap-3">
        <button
          onClick={() => setDialogOpen(true)}
          className="inline-flex items-center gap-2 rounded bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground transition-opacity hover:opacity-90"
        >
          <Download className="h-4 w-4" />
          Download {release.tag_name}
        </button>
        <a
          href={release.html_url}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex items-center gap-2 rounded border border-border px-4 py-2.5 text-sm text-muted-foreground transition-colors hover:border-border/70 hover:text-foreground"
        >
          <ExternalLink className="h-3.5 w-3.5" />
          View release
        </a>
      </div>

      {preferred && (
        <p className="text-xs text-muted-foreground/50 font-mono">
          Detected: {preferred.name}
        </p>
      )}

      <DownloadDialog
        release={release}
        open={dialogOpen}
        onClose={() => setDialogOpen(false)}
      />
    </>
  );
}
