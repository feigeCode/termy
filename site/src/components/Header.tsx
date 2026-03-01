import { Button } from "@/components/ui/button";
import { useTheme } from "@/hooks/useTheme";
import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";

export function Header() {
  const { theme, toggleTheme } = useTheme();
  const [isMobileMenuOpen, setIsMobileMenuOpen] = useState(false);

  const closeMobileMenu = () => setIsMobileMenuOpen(false);
  const toggleMobileMenu = () => setIsMobileMenuOpen((open) => !open);

  useEffect(() => {
    if (!isMobileMenuOpen) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        closeMobileMenu();
      }
    };

    window.addEventListener("keydown", onKeyDown);

    return () => {
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [isMobileMenuOpen]);

  return (
    <header className="fixed top-0 left-0 right-0 z-50 backdrop-blur-xl bg-background/80 border-b border-border/50">
      <nav className="mx-auto flex h-16 max-w-6xl items-center justify-between px-6">
        <Link
          to="/"
          onClick={closeMobileMenu}
          className="flex items-center gap-3 font-semibold text-foreground transition-colors hover:text-primary"
        >
          <div className="relative">
            <img
              src="https://raw.githubusercontent.com/lassejlv/termy/refs/heads/main/assets/termy_icon.png"
              alt="Termy"
              className="h-8 w-8 rounded-lg"
            />
            <div className="absolute -inset-1 rounded-lg bg-primary/20 blur-md -z-10" />
          </div>
          <span className="tracking-tight">Termy</span>
        </Link>

        <div className="hidden items-center gap-1 md:flex">
          <a
            href="#features"
            className="px-4 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground rounded-lg hover:bg-secondary/50"
          >
            Features
          </a>
          <a
            href="#download"
            className="px-4 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground rounded-lg hover:bg-secondary/50"
          >
            Download
          </a>
          <Link
            to="/releases"
            className="px-4 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground rounded-lg hover:bg-secondary/50"
          >
            Releases
          </Link>
          <Link
            to="/docs"
            className="px-4 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground rounded-lg hover:bg-secondary/50"
          >
            Docs
          </Link>
          <a
            href="https://github.com/lassejlv/termy"
            target="_blank"
            rel="noreferrer"
            className="px-4 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground rounded-lg hover:bg-secondary/50"
          >
            GitHub
          </a>
          <div className="w-px h-6 bg-border mx-2" />
          <Button
            variant="ghost"
            size="sm"
            onClick={toggleTheme}
            className="text-muted-foreground hover:text-foreground"
          >
            {theme === "light" ? (
              <svg
                className="w-4 h-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M20.354 15.354A9 9 0 018.646 3.646 9.003 9.003 0 0012 21a9.003 9.003 0 008.354-5.646z"
                />
              </svg>
            ) : (
              <svg
                className="w-4 h-4"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z"
                />
              </svg>
            )}
          </Button>
        </div>

        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={toggleMobileMenu}
          aria-label={isMobileMenuOpen ? "Close menu" : "Open menu"}
          aria-expanded={isMobileMenuOpen}
          aria-controls="mobile-menu"
          className="text-muted-foreground hover:text-foreground md:hidden"
        >
          {isMobileMenuOpen ? (
            <svg
              className="h-5 w-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          ) : (
            <svg
              className="h-5 w-5"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M4 6h16M4 12h16M4 18h16"
              />
            </svg>
          )}
        </Button>
      </nav>

      <button
        type="button"
        aria-label="Close menu"
        onClick={closeMobileMenu}
        className={`fixed inset-0 top-16 z-40 bg-black/20 transition-opacity duration-200 md:hidden ${
          isMobileMenuOpen ? "opacity-100" : "pointer-events-none opacity-0"
        }`}
      />

      <div
        id="mobile-menu"
        aria-hidden={!isMobileMenuOpen}
        className={`absolute left-0 right-0 top-16 z-50 border-t border-border/50 bg-background/95 px-6 py-4 backdrop-blur-xl transition-all duration-200 md:hidden ${
          isMobileMenuOpen
            ? "translate-y-0 opacity-100"
            : "pointer-events-none -translate-y-2 opacity-0"
        }`}
      >
        <div className="flex flex-col gap-1">
          <a
            href="#features"
            onClick={closeMobileMenu}
            className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
          >
            Features
          </a>
          <a
            href="#download"
            onClick={closeMobileMenu}
            className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
          >
            Download
          </a>
          <Link
            to="/releases"
            onClick={closeMobileMenu}
            className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
          >
            Releases
          </Link>
          <Link
            to="/docs"
            onClick={closeMobileMenu}
            className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
          >
            Docs
          </Link>
          <a
            href="https://github.com/lassejlv/termy"
            target="_blank"
            rel="noreferrer"
            onClick={closeMobileMenu}
            className="rounded-lg px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-secondary/50 hover:text-foreground"
          >
            GitHub
          </a>
          <div className="my-2 h-px bg-border/70" />
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={toggleTheme}
            className="w-fit text-muted-foreground hover:text-foreground"
          >
            {theme === "light" ? "Dark mode" : "Light mode"}
          </Button>
        </div>
      </div>
    </header>
  );
}
