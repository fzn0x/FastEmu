# FastEmu's logo

| File | Use |
| --- | --- |
| `icon.svg` | The app icon: a teal tile with a phone whose screen shows a fast-forward sign. Every packaged icon comes from it through `cargo xtask icons`. |
| `logo-light.svg` | The icon and the word FastEmu for light backgrounds. |
| `logo-dark.svg` | The same for dark backgrounds. |
| `logo.svg` | The fallback where a page cannot choose by color scheme: the light version. |

Colors: teal `#2DD4BF` (the tile, the screen frame and the keys), ink
`#0B0F10` (the phone and the word on light backgrounds), white `#FFFFFF`
(the fast-forward sign), paper `#F5F7F7` (the word on dark backgrounds).

The word is set in [Inter](https://github.com/rsms/inter) (SIL Open Font
License 1.1) at its display size, bold, tracked in by 2 percent, with its
letters turned into paths so the files need no font.

The mark began as a concept made with GPT Image 2.6 through Higgsfield
(job `ff03f3e3-4870-4470-bcdc-c29581fe149c`) and was redrawn by hand: the
fast-forward sign now sits on a grid of 64-unit cells, so both chevrons are
the same and centered on the screen.

To change an icon size or add a platform, edit `tools/xtask/src/icons.rs`;
to change the mark, edit `icon.svg` and the lockups together, then run
`cargo xtask icons` and commit what it writes.
