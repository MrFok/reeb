#!/usr/bin/env python3
"""Render the two-bar README chart from the supplied idle working-set readings."""

import base64
import json
import math
from datetime import date
from html import escape
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / 'assets/benchmarks/memory-comparison.json'
OUTPUT = DATA.with_suffix('.svg')


def render():
    data = json.loads(DATA.read_text())
    apps = data['apps']
    metric = next(m for m in data['metrics'] if m['id'] == 'idle_working_set')
    if data['platform'] != 'Windows' or data['unit'] != 'MB':
        raise ValueError('Expected Windows memory readings in MB')
    if [app['id'] for app in apps] != ['keeby', 'reeb']:
        raise ValueError('Expected Keeby and reeb branding in chart order')
    if any(metric[app['id']] <= 0 for app in apps):
        raise ValueError('Working-set readings must be positive')
    maximum = math.ceil(max(metric[app['id']] for app in apps) / 200) * 200
    reduction = (1 - metric['reeb'] / metric['keeby']) * 100
    reported_date = date.fromisoformat(data['reported_on']).strftime('%d %b %Y')
    fonts = {
        weight: base64.b64encode((ROOT / f'assets/ui/Jakarta-{name}.ttf').read_bytes()).decode()
        for weight, name in [(500, 'Medium'), (600, 'SemiBold')]
    }
    svg = [f'''<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="800" height="316" viewBox="0 0 800 316" role="img" aria-labelledby="title desc">
  <title id="title">Windows idle memory usage: Keeby and reeb</title>
  <desc id="desc">Idle working set: Keeby {metric['keeby']:g} MB; reeb {metric['reeb']:g} MB. Reeb uses {reduction:.1f}% less resident RAM in the supplied Windows benchmark. Ten seconds to settle, then median of five samples. Reported on {reported_date}. Both bars use the same scale and zero baseline.</desc>
  <style>
    @font-face {{ font-family: 'Plus Jakarta Sans'; font-style: normal; font-weight: 500; src: url(data:font/ttf;base64,{fonts[500]}) format('truetype'); }}
    @font-face {{ font-family: 'Plus Jakarta Sans'; font-style: normal; font-weight: 600; src: url(data:font/ttf;base64,{fonts[600]}) format('truetype'); }}
    text {{ font-family: 'Plus Jakarta Sans', sans-serif; font-weight: 500; fill: #24292f; }}
    .muted {{ fill: #59636e; }}
    .grid {{ stroke: #e5e9ef; }}
    .card {{ fill: #ffffff; stroke: #d8dee4; }}
    @media (prefers-color-scheme: dark) {{
      text {{ fill: #f0f3f6; }}
      .muted {{ fill: #a9b3bf; }}
      .grid {{ stroke: #303943; }}
      .card {{ fill: #151b23; stroke: #3d444d; }}
    }}
  </style>
  <rect class="card" x="0.5" y="0.5" width="799" height="315" rx="18"/>
  <text x="32" y="49" font-size="25" font-weight="600">Memory usage</text>
  <text class="muted" x="32" y="76" font-size="14">Windows · idle working set · lower is better</text>
''']
    for tick in range(0, maximum + 1, 200):
        x = 230 + tick / maximum * 430
        svg.append(f'<path class="grid" d="M{x} 104V267"/>')
        label = f'{tick} MB' if tick == maximum else str(tick)
        svg.append(f'<text class="muted" x="{x}" y="291" text-anchor="middle" font-size="12">{label}</text>')
    for index, app in enumerate(reversed(apps)):
        y = 116 + index * 88
        logo = base64.b64encode((ROOT / app['logo']).read_bytes()).decode()
        value = metric[app['id']]
        width = value / maximum * 430
        svg.append(f'''  <image x="32" y="{y}" width="48" height="48" xlink:href="data:image/png;base64,{logo}"/>
  <text x="94" y="{y + 23}" font-size="19" font-weight="600">{escape(app['name'])}</text>
  <text class="muted" x="94" y="{y + 44}" font-size="12">Windows</text>
  <rect data-app="{app['id']}" x="230" y="{y + 7}" width="{width:.3f}" height="34" rx="5" fill="{app['color']}"/>
  <text x="{230 + width + 12:.3f}" y="{y + 30}" font-size="18" font-weight="600">{value:g} MB</text>''')
    svg.append('</svg>\n')
    OUTPUT.write_text('\n'.join(svg))


if __name__ == '__main__':
    render()
