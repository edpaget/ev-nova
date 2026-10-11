# Bundled fonts

`NotoSans-Regular.ttf` is Noto Sans Regular, Copyright 2012 Google Inc.,
licensed under the SIL Open Font License 1.1 (see `OFL.txt`). It is
unmodified: copied byte for byte from the `fonts/` directory of the
`cosmic-text` 0.19.0 crate.

- sha256: `2ec33f84606cbaa0a1a944488e14f97faf2f6a25ecdd8354f5358f06da13c7d9`
- size: 556,216 bytes

`nova-render` compiles it in as the substitute for Geneva, and for Charcoal
when the player's game data has no usable `Fonts/Charcoal.ttf`. The OFL
lets it ship with this GPL-licensed program as long as `OFL.txt` travels
with it. It is not subset or renamed.

## Noto Serif

`NotoSerif-Regular.ttf` is Noto Serif Regular, version 2.015, Copyright
2022 The Noto Project Authors, licensed under the SIL Open Font License
1.1 (see `NotoSerif-OFL.txt`, the licence from the same release). It is
unmodified: copied byte for byte from `NotoSerif/unhinted/ttf/` in the
Noto project's release archive,
<https://github.com/notofonts/latin-greek-cyrillic/releases/download/NotoSerif-v2.015/NotoSerif-v2.015.zip>.

- sha256: `a15cfbbc1539d707115111d672d590a3d70d4f74b4c0a315956da20ae19a14e1`
- size: 482,540 bytes

`nova-render` compiles it in as the substitute for QuickDraw font 20,
Times, which titles the Shipyard Info panel and which EV Nova.app does
not ship. The OFL lets it ship with this GPL-licensed program as long as
`NotoSerif-OFL.txt` travels with it. It is not subset or renamed.
