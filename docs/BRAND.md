# Rarog visual identity

The Rarog bird-and-orbit mark is the visual identity for the engine, its repository and Rarog-branded applications.

## Canonical assets

- `assets/branding/rarog-icon.webp` — lossless repository/display derivative of the approved mark.
- `assets/branding/rarog-window-icon-24.rgba` — generated 24×24 RGBA runtime derivative used by the current Windows GUI shell.

Application packaging should generate platform-native icon sets from the same approved master artwork. Do not redraw the bird, replace the orbit, alter the composition, or use a different project mark.

## Palette

| Token | Hex | Role |
| --- | --- | --- |
| Ember Navy | `#0B0E2C` | primary dark surface/background |
| Wing Indigo | `#3A44AE` | cool structural accent |
| Orbit Purple | `#692B51` | secondary accent/transition |
| Flame Red | `#EC2820` | strong active/emphasis state |
| Rarog Orange | `#F95023` | primary brand accent |
| Solar Gold | `#FDC94B` | highlight and focal glow |

The visual hierarchy should normally move from deep navy foundations through indigo/purple structure to red/orange/gold emphasis. Avoid unrelated accent palettes when a Rarog brand color serves the same purpose.

## Repository presentation

The README, project artwork and milestone/release presentation should use the canonical mark and the palette above. GitHub's own interface remains native; the goal is a consistent identity inside repository-controlled surfaces rather than imitation of custom application chrome.

Technical documentation stays restrained: brand styling must not reduce scanability, code readability or contrast.

## Application presentation

Rarog-branded applications should use the canonical mark as their application/window/package icon. The current Windows `rarog-window` shell embeds a generated RGBA derivative and supplies it to winit when the window is created.

Future Windows packaging should carry a multi-resolution ICO derived from the same artwork. Future macOS/iOS and Android packaging should likewise derive their required native icon assets from the same master while respecting each platform's icon safe-area and packaging rules.

Dark navy surfaces, fire-colored primary emphasis and indigo/purple secondary accents are the default visual language for future Rarog-owned UI. Native platform conventions, accessibility and sufficient text/control contrast remain mandatory.

## Mark handling

Do:

- preserve the square composition and rounded-corner artwork;
- preserve the bird/orbit relationship and original color treatment;
- keep clear space around the mark;
- derive smaller/native assets from the approved master.

Do not:

- recolor or monochrome the mark as a default;
- stretch, crop through the bird, rotate or mirror it;
- add text inside the icon;
- add another logo next to or over the bird;
- use arbitrary gradients or glows that compete with the mark.

When a new visual asset is needed, start from this identity rather than inventing a separate style.
