# Windows memory comparison

The README chart shows two bars for idle working set from the Windows benchmark supplied by the project owner on 23 October 2026. It replaces the earlier approximate 354 MB versus 60–68 MB readings. Other supplied metrics are retained below for reference.

| Metric | reeb | Keeby | Reduction with reeb |
| --- | ---: | ---: | ---: |
| Idle working set | 132.9 MB | 502.4 MB | 73.5% |
| Idle private memory | 95.3 MB | 419.1 MB | 77.3% |
| Peak working set | 158.5 MB | 521.9 MB | 69.6% |
| Working set after 120 typed keys | ~127 MB | ~478 MB | ~73.4% |

Working set measures resident RAM. Private memory is a separate counter and is shown separately. The two chart bars use a zero baseline and the same 0–600 MB scale. After-typing readings are approximate because the supplied screenshot shows only whole MB for that condition.

The reductions are calculated as `(1 - reeb / Keeby) × 100`. The headline idle working-set comparison supports **73.5% less resident RAM in this test**, replacing the earlier 81–83% figure.

## Measurement context

The supplied report describes a test on one Windows machine:

- Idle: allow 10 seconds to settle, then take five samples and report the median.
- Keeby: version 1.8.1, .NET/WPF offline copy.
- Reeb: Rust/egui build from `dist`; version not provided.
- Typing workload: 120 keys.

The report also lists these resource readings:

| Resource | reeb | Keeby |
| --- | ---: | ---: |
| Threads | 36 | 100 |
| Handles | 339 | 967 |
| Application folder | 10.2 MB | 224.8 MB |
| Executable | 8.7 MB | Not supplied |
| Files | Not supplied | 1,153 |

These figures were transcribed from the supplied screenshot, not independently rerun in this chat. The screenshot does not specify the Windows version, hardware, window state, sound settings, underlying byte-to-MB conversion, raw samples or peak-observation duration. Idle sampling details apply to idle metrics; the report does not separately document sampling after typing.

Both reported working sets after 120 keys were below their idle values. This short test alone does not establish the absence of long-term memory growth. Results describe this Windows test, not all configurations or macOS.

## Data and rendering

[Chart data](../assets/benchmarks/memory-comparison.json) preserves the transcribed readings, method, source screenshot filename and SHA-256. Earlier Mac measurements remain separate in [the macOS sample data](../assets/benchmarks/memory-comparison-macos.json).

Regenerate the SVG with Python's standard library:

```sh
python3 scripts/render-memory-comparison.py
```

## Branding

Keeby's logo comes from its [official website asset](https://getkeeby.com/keeby-logo.webp), retrieved on 22 October 2026 and converted losslessly to PNG for SVG compatibility. The orange bar (`#FB8718`) is sampled from that logo. The mark identifies the app being compared; no affiliation or endorsement is implied.

Reeb uses its existing project icon and the blue accent from `src/theme.rs` (`#2865E8`). Both logos and the app's Plus Jakarta Sans Medium and SemiBold fonts are embedded in the SVG, so it needs no external requests or scripts. The font's license is in `assets/ui/OFL.txt`. The chart supports light and dark backgrounds and includes accessible title and description text.
