# Bundled fonts

Static faces of the three typefaces in `docs/DESIGN.md`, at the weights it
uses. `src/fonts.rs` builds them into the binary and registers them at start.
They exist because the pinned GPUI ignores a variable font's weight axis, so
the variable fonts most systems install draw every weight at their default
(Regular for Inter and JetBrains Mono, Light for Space Grotesk).

| File | Weight | Licence |
|---|---|---|
| `Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-SemiBold.ttf` | 400, 500, 600 | `OFL-Inter.txt` |
| `SpaceGrotesk-SemiBold.ttf` | 600 | `OFL-SpaceGrotesk.txt` |
| `JetBrainsMono-Regular.ttf`, `JetBrainsMono-Medium.ttf` | 400, 500 | `OFL-JetBrainsMono.txt` |

All three are under the SIL Open Font License 1.1, which allows bundling them
with software; none declares a Reserved Font Name, so the instances keep the
families' names. The licence files must ship with the fonts.

## How they were made

They are named instances of the variable fonts in the Google Fonts
collection (Inter 4.001, Space Grotesk 2.000, JetBrains Mono 2.211), cut with
fontTools 4.68:

```bash
python3 -m fontTools.varLib.instancer --update-name-table -o Inter-SemiBold.ttf 'Inter[opsz,wght].ttf' wght=600 opsz=14
python3 -m fontTools.varLib.instancer --update-name-table -o JetBrainsMono-Medium.ttf 'JetBrainsMono[wght].ttf' wght=500
```

and likewise for the other Inter and JetBrains Mono weights. Inter's optical
size is pinned at 14, its text cut. Space Grotesk's variable font has no
style name for 600, so it was cut without `--update-name-table`; its name
table was then set by hand (family "Space Grotesk", subfamily "SemiBold",
PostScript name `SpaceGrotesk-SemiBold`), its `STAT` table dropped and its
OS/2 weight class set to 600.

To add a weight, cut it the same way, add it to `FACES` in `src/fonts.rs`
and to the test's list of weights in use.
