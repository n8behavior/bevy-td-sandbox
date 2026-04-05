# Credits

## Audio

Sound effects are currently placeholder sine wave tones generated using Bevy's built-in `Pitch` audio source. Each sound uses a distinct frequency to represent its game event.

To replace with real audio assets, source CC0 or CC-BY licensed `.ogg`/`.wav` files from [freesound.org](https://freesound.org) and update the `SoundAssets` resource in `src/audio/` to load them via `AssetServer` instead of `Pitch`.

If using CC-BY licensed audio, add attribution for each file below:

<!-- Example:
- tower_scrapgun.ogg — "Metal Ping" by AuthorName (freesound.org/s/12345) — CC-BY 4.0
-->
