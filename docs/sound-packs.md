# Bundled sounds

All recordings are embedded in the executable. The catalog has 16 profiles and 187 mono PCM16 WAV recordings at 44.1 kHz.

| Profile | Recordings | Published license |
| --- | ---: | --- |
| NovelKeys Cream | 12 | MIT |
| Alpaca | 12 | MIT |
| Gateron Black Ink | 12 | MIT |
| Gateron Red Ink | 12 | MIT |
| Holy Panda | 12 | MIT |
| Cherry MX Black | 12 | MIT |
| Cherry MX Blue | 6 | MIT |
| Cherry MX Brown | 12 | MIT |
| Kailh Box Navy | 12 | MIT |
| Buckling Spring | 12 | MIT |
| SKCM Blue Alps | 12 | MIT |
| Topre | 12 | MIT |
| Turquoise Tealios | 12 | MIT |
| Cherry MX Brown · Foxfire | 2 | CC0-1.0 |
| Cherry KC 1000 | 32 | CC0-1.0 |
| MacBook Air M2 | 3 | CC0-1.0 |

The 13 kbsim profiles are distributed under the upstream repository's MIT license. The three additional profiles have CC0 grants on their creators' asset pages. Retain the [notices](../assets/SOUND-NOTICES.txt), [MIT text](../assets/KBSIM-LICENSE.txt) and [CC0 text](../assets/CC0-LICENSE.txt) with distributed copies. Published licenses are the basis for redistribution; the repository does not document every original recording session.

The [manifest](../assets/sound-packs.json) records pinned source URLs, source hashes, WAV hashes, conversion details and license-evidence links. Missing special-key clips fall back to the same profile's generic recordings. KC 1000 and MacBook Air M2 play complete keystrokes on key-down only, without duplicate release sounds.

Run `python scripts/verify-assets.py` to check every WAV hash, format and manifest entry. Add new sources only with a published redistribution grant, required notices, conversion details and hashes.
