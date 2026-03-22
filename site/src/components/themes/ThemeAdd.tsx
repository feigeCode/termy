import { useState, useEffect } from "react";
import { ArrowLeft, Upload } from "lucide-react";
import { fetchCurrentUser, createTheme, getThemeLoginUrl } from "../../lib/theme-store";
import type { AuthUser } from "../../lib/theme-store";

export default function ThemeAdd() {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [loading, setLoading] = useState(true);
  const [form, setForm] = useState({ name: "", description: "", version: "1.0.0", isPublic: true, themeJson: "" });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  useEffect(() => {
    fetchCurrentUser().then(setUser).catch(() => setUser(null)).finally(() => setLoading(false));
  }, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError(null);
    try {
      await createTheme(form);
      setSuccess(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create theme");
    } finally {
      setSubmitting(false);
    }
  }

  if (loading) return <div className="mx-auto max-w-lg px-4 py-20 text-center text-muted-foreground text-sm">Loading...</div>;

  if (!user) {
    return (
      <div className="mx-auto max-w-lg px-4 py-20 text-center">
        <p className="text-muted-foreground mb-4">Sign in to publish a theme.</p>
        <a href={getThemeLoginUrl("/themes/add")} className="inline-flex items-center gap-2 rounded bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground">
          Sign in with GitHub
        </a>
      </div>
    );
  }

  if (success) {
    return (
      <div className="mx-auto max-w-lg px-4 py-20 text-center">
        <p className="text-foreground font-semibold mb-2">Theme published!</p>
        <a href="/themes" className="text-sm text-primary hover:underline">Back to themes</a>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-lg px-4 sm:px-6 py-12">
      <a href="/themes" className="inline-flex items-center gap-1.5 mb-8 text-sm text-muted-foreground hover:text-foreground">
        <ArrowLeft className="h-4 w-4" />
        All themes
      </a>
      <h1 className="text-2xl font-bold mb-6">Add a theme</h1>
      <form onSubmit={handleSubmit} className="space-y-4">
        {[
          { label: "Name", key: "name", type: "text", required: true },
          { label: "Version", key: "version", type: "text", required: true },
        ].map(({ label, key, type, required }) => (
          <div key={key}>
            <label className="block text-sm font-medium text-foreground mb-1.5">{label}</label>
            <input
              type={type}
              required={required}
              value={(form as any)[key]}
              onChange={(e) => setForm((f) => ({ ...f, [key]: e.target.value }))}
              className="w-full rounded border border-border bg-card/40 px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
            />
          </div>
        ))}
        <div>
          <label className="block text-sm font-medium text-foreground mb-1.5">Description</label>
          <textarea
            value={form.description}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
            rows={3}
            className="w-full rounded border border-border bg-card/40 px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-ring resize-none"
          />
        </div>
        <div>
          <label className="block text-sm font-medium text-foreground mb-1.5">Theme JSON</label>
          <textarea
            value={form.themeJson}
            onChange={(e) => setForm((f) => ({ ...f, themeJson: e.target.value }))}
            rows={8}
            placeholder='{ "background": "#0a0a0a", ... }'
            className="w-full rounded border border-border bg-card/40 px-3 py-2 text-sm font-mono focus:outline-none focus:ring-1 focus:ring-ring resize-none"
          />
        </div>
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            id="isPublic"
            checked={form.isPublic}
            onChange={(e) => setForm((f) => ({ ...f, isPublic: e.target.checked }))}
            className="rounded"
          />
          <label htmlFor="isPublic" className="text-sm text-muted-foreground">Make publicly visible</label>
        </div>
        {error && <p className="text-sm text-destructive">{error}</p>}
        <button
          type="submit"
          disabled={submitting}
          className="inline-flex items-center gap-2 rounded bg-primary px-4 py-2.5 text-sm font-semibold text-primary-foreground disabled:opacity-50 transition-opacity hover:opacity-90"
        >
          <Upload className="h-4 w-4" />
          {submitting ? "Publishing..." : "Publish theme"}
        </button>
      </form>
    </div>
  );
}
