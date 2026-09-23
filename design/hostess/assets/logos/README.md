# Logos

The Hostess mark and app icon. Every file is an SVG on the 1024 grid with the macOS squircle (a 824px rect at rx 186 inside the 1024 canvas — Xcode's icon composer accepts the same geometry).

| File | Use |
| --- | --- |
| `hostess-app-icon-light.svg` | The default app icon: the ivory mark on flat clay. Dock, Finder, Launchpad, About. |
| `hostess-app-icon-dark.svg` | Dark appearance. Deeper clay; the mark is unchanged. |
| `hostess-app-icon-clear.svg` | Clear appearance: translucent white that takes the wallpaper, with the mark in slate. |
| `hostess-app-icon-tinted.svg` | Tinted appearance: warm grey for macOS to tint. |
| `hostess-menubar-template.svg` | The status item. Single ink `#000000`, exported as a template image so macOS recolours it; draw it at 16pt. |

Rules: never redraw the mark by hand; never colour the template glyph in code (set `isTemplate`); never add a badge to the app icon — badges belong to the status item only.
