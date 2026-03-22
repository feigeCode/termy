import type { APIRoute } from "astro";
import { getCollection } from "astro:content";

export const GET: APIRoute = async () => {
  // Fetch latest release
  let latestVersion = "latest";
  for (const url of [
    "https://api.termy.run/api/github/releases/latest",
    "https://api.github.com/repos/lassejlv/termy/releases/latest",
  ]) {
    try {
      const r = await fetch(url);
      if (r.ok) {
        const data = await r.json();
        latestVersion = data.tag_name ?? latestVersion;
        break;
      }
    } catch {
      // try next
    }
  }

  // Fetch docs list from content collection
  const docs = await getCollection("docs");
  const sortedDocs = docs
    .filter((d) => d.data.category?.toLowerCase() !== "plugins")
    .sort((a, b) => (a.data.order ?? 99) - (b.data.order ?? 99));

  const docsIndex = sortedDocs
    .map((d) => `- ${d.data.title ?? d.id}: /docs/${d.id}`)
    .join("\n");

  const body = `# Termy

> A fast, GPU-accelerated terminal emulator built with Rust and GPUI.

## Overview

Termy is an open-source terminal emulator focused on performance and simplicity. It's built using Rust for the backend and GPUI (Zed's GPU-accelerated UI framework) for rendering.

Latest release: ${latestVersion}

## Links

- Website: https://termy.sh
- GitHub: https://github.com/lassejlv/termy
- Discord: https://discord.gg/termy
- Releases: https://termy.sh/releases
- Documentation: https://termy.sh/docs
- Themes: https://termy.sh/themes

## Installation

### macOS (Homebrew)
\`\`\`
brew tap lassejlv/termy https://github.com/lassejlv/termy
brew install --cask termy
\`\`\`

Termy is not code signed yet. If Gatekeeper blocks launch after moving Termy to \`Applications\`, run:
\`\`\`
sudo xattr -d com.apple.quarantine /Applications/Termy.app
\`\`\`

### Windows
Install using the latest \`Setup.exe\` release asset.

If SmartScreen appears on first launch:
1. Click \`More info\`
2. Click \`Run anyway\`

### Linux (AppImage)
\`\`\`
chmod +x Termy-*.AppImage
./Termy-*.AppImage
\`\`\`

### Linux (Tarball)
\`\`\`
tar -xzf Termy-*.tar.gz
cd termy
./install.sh
\`\`\`

### Arch Linux
\`\`\`
paru -S termy-bin
\`\`\`

## Updating

Use the Command Palette and run \`Check for updates\`.

- macOS: \`cmd+p\`
- Windows/Linux: \`ctrl+p\`

---

# Documentation

## Docs Index

${docsIndex}

---

## Configuration

Termy reads configuration from \`~/.config/termy/config.txt\`.

### Appearance

\`theme\`
- Default: \`termy\`
- Current color scheme name

\`font_family\`
- Default: \`JetBrains Mono\`
- Font family used in terminal UI

\`font_size\`
- Default: \`14\`
- Terminal font size in pixels

\`background_opacity\`
- Default: \`1\`
- Window background opacity (0.0 to 1.0)

\`background_blur\`
- Default: \`false\`
- Enable blur effect for transparent backgrounds

\`padding_x\`
- Default: \`12\`
- Left and right terminal padding

\`padding_y\`
- Default: \`8\`
- Top and bottom terminal padding

### Terminal

\`shell\`
- Default: unset
- Executable used for new sessions

\`term\`
- Default: \`xterm-256color\`
- TERM value exposed to child applications

\`colorterm\`
- Default: \`truecolor\`
- COLORTERM value exposed to child applications

\`cursor_style\`
- Default: \`block\`
- Shape of the terminal cursor

\`cursor_blink\`
- Default: \`true\`
- Enable blinking cursor animation

\`mouse_scroll_multiplier\`
- Default: \`3\`
- Mouse wheel scroll speed multiplier

\`scrollbar_visibility\`
- Default: \`on_scroll\`
- Terminal scrollbar visibility behavior

\`scrollbar_style\`
- Default: \`neutral\`
- Terminal scrollbar color style

\`scrollback_history\`
- Default: \`2000\`
- Aliases: \`scrollback\`
- Lines retained in terminal scrollback

\`inactive_tab_scrollback\`
- Default: unset
- Scrollback limit for inactive tabs

\`command_palette_show_keybinds\`
- Default: \`true\`
- Show shortcut badges in command palette rows

### Tabs

\`tab_title_priority\`
- Default: \`manual, explicit, shell, fallback\`
- Exact source priority for tab titles

\`tab_title_mode\`
- Default: \`smart\`
- How tab titles are determined

\`tab_title_fallback\`
- Default: \`Terminal\`
- Default tab title when no source is available

\`tab_title_explicit_prefix\`
- Default: \`termy:tab:\`
- Prefix used for explicit OSC title payloads

\`tab_title_shell_integration\`
- Default: \`true\`
- Export TERMY_* environment values for shell hooks

\`tab_title_prompt_format\`
- Default: \`{cwd}\`
- Template for prompt-derived tab titles

\`tab_title_command_format\`
- Default: \`{command}\`
- Template for command-derived tab titles

\`tab_close_visibility\`
- Default: \`active_hover\`
- When tab close buttons are visible

\`tab_width_mode\`
- Default: \`active_grow_sticky\`
- How tab widths react to active state

\`show_termy_in_titlebar\`
- Default: \`true\`
- Show or hide the termy branding in the titlebar

### Advanced

\`working_dir\`
- Default: unset
- Initial directory for new sessions

\`working_dir_fallback\`
- Default: \`home\` (macOS/Windows), \`process\` (Linux/other)
- Aliases: \`default_working_dir\`
- Directory used when working_dir is unset

\`warn_on_quit_with_running_process\`
- Default: \`true\`
- Warn before quitting when a tab has an active process

\`window_width\`
- Default: \`1280\`
- Default startup window width in pixels

\`window_height\`
- Default: \`820\`
- Default startup window height in pixels

### Colors

Use \`[colors]\` to override theme colors with \`#RRGGBB\` values.

Available color keys:
- \`foreground\` (alias: \`fg\`) - Default text color
- \`background\` (alias: \`bg\`) - Terminal background color
- \`cursor\` - Cursor color
- \`black\` (alias: \`color0\`) - ANSI black
- \`red\` (alias: \`color1\`) - ANSI red
- \`green\` (alias: \`color2\`) - ANSI green
- \`yellow\` (alias: \`color3\`) - ANSI yellow
- \`blue\` (alias: \`color4\`) - ANSI blue
- \`magenta\` (alias: \`color5\`) - ANSI magenta
- \`cyan\` (alias: \`color6\`) - ANSI cyan
- \`white\` (alias: \`color7\`) - ANSI white
- \`bright_black\` (aliases: \`brightblack\`, \`color8\`) - ANSI bright black
- \`bright_red\` (aliases: \`brightred\`, \`color9\`) - ANSI bright red
- \`bright_green\` (aliases: \`brightgreen\`, \`color10\`) - ANSI bright green
- \`bright_yellow\` (aliases: \`brightyellow\`, \`color11\`) - ANSI bright yellow
- \`bright_blue\` (aliases: \`brightblue\`, \`color12\`) - ANSI bright blue
- \`bright_magenta\` (aliases: \`brightmagenta\`, \`color13\`) - ANSI bright magenta
- \`bright_cyan\` (aliases: \`brightcyan\`, \`color14\`) - ANSI bright cyan
- \`bright_white\` (aliases: \`brightwhite\`, \`color15\`) - ANSI bright white

---

## Keybindings

Termy keybindings use Ghostty-style trigger overrides via repeated \`keybind\` lines in \`~/.config/termy/config.txt\`.

### Default Keybinds

#### macOS Defaults
- \`cmd-q\` -> \`quit\`
- \`cmd-,\` -> \`open_settings\`
- \`cmd-p\` -> \`toggle_command_palette\`
- \`cmd-t\` -> \`new_tab\`
- \`cmd-w\` -> \`close_tab\`
- \`cmd-=\` -> \`zoom_in\`
- \`cmd-+\` -> \`zoom_in\`
- \`cmd--\` -> \`zoom_out\`
- \`cmd-0\` -> \`zoom_reset\`
- \`cmd-f\` -> \`open_search\`
- \`cmd-g\` -> \`search_next\`
- \`cmd-shift-g\` -> \`search_previous\`
- \`cmd-m\` -> \`minimize_window\`
- \`cmd-c\` -> \`copy\`
- \`cmd-v\` -> \`paste\`

#### Windows/Linux Defaults
- \`ctrl-q\` -> \`quit\`
- \`ctrl-,\` -> \`open_settings\`
- \`ctrl-p\` -> \`toggle_command_palette\`
- \`ctrl-t\` -> \`new_tab\`
- \`ctrl-w\` -> \`close_tab\`
- \`ctrl-=\` -> \`zoom_in\`
- \`ctrl-+\` -> \`zoom_in\`
- \`ctrl--\` -> \`zoom_out\`
- \`ctrl-0\` -> \`zoom_reset\`
- \`ctrl-f\` -> \`open_search\`
- \`ctrl-g\` -> \`search_next\`
- \`ctrl-shift-g\` -> \`search_previous\`
- \`ctrl-shift-c\` -> \`copy\`
- \`ctrl-shift-v\` -> \`paste\`

Note: \`secondary\` maps to \`cmd\` on macOS and \`ctrl\` on non-macOS platforms.

---

## Example Config

\`\`\`
# ~/.config/termy/config.txt

# Appearance
theme = dracula
font_family = Fira Code
font_size = 13
background_opacity = 0.95
background_blur = true

# Terminal
cursor_style = beam
cursor_blink = false
scrollback_history = 5000

# Tabs
tab_title_mode = smart
show_termy_in_titlebar = false

# Keybindings
keybind = cmd-shift-t=new_tab
keybind = cmd-shift-w=close_tab

# Colors
[colors]
foreground = #f8f8f2
background = #282a36
cursor = #f8f8f2
\`\`\`

---

## License

Open source - see GitHub repository for license details.
`;

  return new Response(body, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "public, max-age=3600, stale-while-revalidate=7200",
    },
  });
};
