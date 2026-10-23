# App icon

- `icon-default.png`: the icon (1024×1024, rounded, transparent corners).
- `android-fg.png`, `android-bg.png`: the two layers of the Android adaptive
  icon (the lotus inside the safe zone, on the cream background).
- `play-store-512.png`: the icon for the Google Play listing (512×512, square;
  Play rounds the corners itself).

To regenerate every size (desktop, Windows, iOS, Android) after a change:

    npx tauri icon tools/icon/manifest.json

then copy `icon-default.png` to `static/app-icon.png` (256×256) and
`static/favicon.png` (64×64). The Android icons for versions before 8
(`mipmap-*/ic_launcher.png` and `ic_launcher_round.png`) were made from the
rounded icon at 44/48 of their size, so that the lotus is not too small.
