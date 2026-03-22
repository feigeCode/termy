import { useState } from "react";
import { fallbackPalette } from "../../lib/theme-store";
import type { ThemePalette } from "../../lib/theme-store";
import { Copy, Check } from "lucide-react";

const COLOR_FIELDS: Array<{ key: keyof ThemePalette; label: string }> = [
  { key: "background", label: "Background" },
  { key: "foreground", label: "Foreground" },
  { key: "cursor", label: "Cursor" },
  { key: "black", label: "Black" },
  { key: "red", label: "Red" },
  { key: "green", label: "Green" },
  { key: "yellow", label: "Yellow" },
  { key: "blue", label: "Blue" },
  { key: "magenta", label: "Magenta" },
  { key: "cyan", label: "Cyan" },
  { key: "white", label: "White" },
  { key: "bright_black", label: "Bright Black" },
  { key: "bright_red", label: "Bright Red" },
  { key: "bright_green", label: "Bright Green" },
  { key: "bright_yellow", label: "Bright Yellow" },
  { key: "bright_blue", label: "Bright Blue" },
  { key: "bright_magenta", label: "Bright Magenta" },
  { key: "bright_cyan", label: "Bright Cyan" },
  { key: "bright_white", label: "Bright White" },
];

export default function ThemeStudio() {
  const [palette, setPalette] = useState<Required<ThemePalette>>(fallbackPalette);
  const [copied, setCopied] = useState(false);

  const json = JSON.stringify(palette, null, 2);

  function copyJson() {
    navigator.clipboard.writeText(json).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  }

  return (
    <div className="mx-auto max-w-6xl px-4 sm:px-6 py-12">
      <div className="mb-8">
        <h1 className="text-3xl font-bold tracking-tight">Theme Studio</h1>
        <p className="mt-1.5 text-muted-foreground">Design your Termy color scheme.</p>
      </div>

      <div className="grid gap-8 lg:grid-cols-2">
        {/* Color pickers */}
        <div className="space-y-2">
          <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-4">Colors</h2>
          <div className="grid grid-cols-2 gap-2">
            {COLOR_FIELDS.map(({ key, label }) => (
              <div key={key} className="flex items-center gap-2 rounded border border-border/50 px-3 py-2">
                <input
                  type="color"
                  value={palette[key]}
                  onChange={(e) => setPalette((p) => ({ ...p, [key]: e.target.value }))}
                  className="h-6 w-6 cursor-pointer rounded border-0 bg-transparent p-0"
                />
                <span className="text-xs text-muted-foreground">{label}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Preview + JSON */}
        <div className="space-y-4">
          {/* Terminal preview */}
          <div>
            <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider mb-4">Preview</h2>
            <div
              className="rounded border overflow-hidden font-mono text-sm"
              style={{ background: palette.background, borderColor: "#333" }}
            >
              <div className="px-3 py-2 border-b flex gap-1.5" style={{ background: palette.black, borderColor: "#333" }}>
                <div className="w-2 h-2 rounded-full" style={{ background: palette.red }} />
                <div className="w-2 h-2 rounded-full" style={{ background: palette.yellow }} />
                <div className="w-2 h-2 rounded-full" style={{ background: palette.green }} />
              </div>
              <div className="p-4 space-y-1.5 text-xs leading-relaxed">
                <div style={{ color: palette.green }}>$ ls -la</div>
                <div style={{ color: palette.blue }}>drwxr-xr-x  src/</div>
                <div style={{ color: palette.cyan }}>-rw-r--r--  config.toml</div>
                <div style={{ color: palette.foreground }}>-rw-r--r--  README.md</div>
                <div style={{ color: palette.yellow }}>$ git status</div>
                <div style={{ color: palette.green }}>On branch main</div>
                <div style={{ color: palette.foreground }}>nothing to commit ✓</div>
                <div style={{ color: palette.cursor }}>█</div>
              </div>
            </div>
          </div>

          {/* JSON output */}
          <div>
            <div className="flex items-center justify-between mb-2">
              <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">JSON</h2>
              <button
                onClick={copyJson}
                className="flex items-center gap-1.5 rounded border border-border/50 px-2.5 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
              >
                {copied ? <Check className="h-3 w-3 text-primary" /> : <Copy className="h-3 w-3" />}
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>
            <pre className="rounded border border-border bg-card/40 p-4 text-xs font-mono text-muted-foreground overflow-auto max-h-64 leading-relaxed">
              {json}
            </pre>
          </div>
        </div>
      </div>
    </div>
  );
}
