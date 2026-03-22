import { Download } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { AppleIcon, WindowsIcon, LinuxIcon } from "@/components/platform-icons";
import type { Release, Asset } from "@/lib/types";
import { formatBytes } from "@/lib/utils";

function classifyAssets(assets: Asset[]) {
  return {
    mac: assets.filter((a) => a.name.toLowerCase().endsWith(".dmg")),
    windows: assets.filter((a) => {
      const n = a.name.toLowerCase();
      return (
        n.endsWith(".exe") ||
        n.endsWith(".msi") ||
        n.includes("windows") ||
        n.includes("win64") ||
        n.includes("pc-windows")
      );
    }),
    linux: assets.filter((a) => {
      const n = a.name.toLowerCase();
      return (
        (n.includes("linux") && n.endsWith(".tar.gz")) ||
        n.endsWith(".appimage") ||
        n.endsWith(".deb") ||
        n.endsWith(".rpm")
      );
    }),
  };
}

interface PlatformSectionProps {
  label: string;
  icon: React.ReactNode;
  assets: Asset[];
}

function PlatformSection({ label, icon, assets }: PlatformSectionProps) {
  if (assets.length === 0) return null;
  return (
    <div>
      <div className="flex items-center gap-2 mb-2">
        <span className="text-muted-foreground/60">{icon}</span>
        <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground/60">
          {label}
        </span>
      </div>
      <div className="space-y-1.5">
        {assets.map((asset) => (
          <a
            key={asset.name}
            href={asset.browser_download_url}
            className="flex items-center justify-between rounded border border-border/50 px-3 py-2.5 text-sm transition-colors hover:border-border hover:bg-card/60 group"
          >
            <span className="font-mono text-xs text-foreground/80 truncate pr-4">
              {asset.name}
            </span>
            <div className="flex items-center gap-3 shrink-0">
              {asset.size > 0 && (
                <span className="text-xs text-muted-foreground/50">
                  {formatBytes(asset.size)}
                </span>
              )}
              <Download className="h-3.5 w-3.5 text-muted-foreground/40 group-hover:text-primary transition-colors" />
            </div>
          </a>
        ))}
      </div>
    </div>
  );
}

interface Props {
  release: Release | null;
  open: boolean;
  onClose: () => void;
}

export default function DownloadDialog({ release, open, onClose }: Props) {
  if (!release) return null;

  const classified = classifyAssets(release.assets);

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-lg bg-background border-border">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 font-mono text-base">
            <Download className="h-4 w-4 text-primary" />
            Download Termy{" "}
            <span className="text-muted-foreground font-normal">
              {release.tag_name}
            </span>
          </DialogTitle>
        </DialogHeader>

        <div className="mt-2 space-y-5">
          <PlatformSection
            label="macOS"
            icon={<AppleIcon className="h-3.5 w-3.5" />}
            assets={classified.mac}
          />
          <PlatformSection
            label="Windows"
            icon={<WindowsIcon className="h-3.5 w-3.5" />}
            assets={classified.windows}
          />
          <PlatformSection
            label="Linux"
            icon={<LinuxIcon imgClassName="h-3.5 w-3.5" />}
            assets={classified.linux}
          />
        </div>

        <div className="mt-4 pt-4 border-t border-border/50">
          <a
            href={release.html_url}
            target="_blank"
            rel="noopener noreferrer"
            className="text-xs text-muted-foreground/60 hover:text-muted-foreground transition-colors"
          >
            View full release on GitHub →
          </a>
        </div>
      </DialogContent>
    </Dialog>
  );
}
