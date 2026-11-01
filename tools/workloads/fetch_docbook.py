#!/usr/bin/env python3
"""Fetch the published DocBook xslTNG 2.8.4 modules corresponding to a checkout."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.error import HTTPError
from urllib.request import urlopen

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('source', type=Path, help='DocBook xslTNG 2.8.4 checkout')
parser.add_argument('destination', type=Path, help='directory for published xslt modules')
args = parser.parse_args()
source = args.source / 'src/main/xslt'
if not source.is_dir():
    parser.error('source checkout has no src/main/xslt directory')
base = 'https://cdn.docbook.org/release/xsltng/2.8.4/release/xslt/'
paths = sorted({p.relative_to(source).as_posix() for p in source.rglob('*.xsl')} | {'param.xsl', 'VERSION.xsl'})


def fetch(path):
    try:
        with urlopen(base + path, timeout=30) as response:
            data = response.read()
    except HTTPError as error:
        if error.code == 404:
            return path
        raise
    target = args.destination / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    return None


with ThreadPoolExecutor(max_workers=8) as pool:
    missing = [p for p in pool.map(fetch, paths) if p]
print(f'Fetched {len(paths) - len(missing)} published modules; absent in release: {", ".join(missing) or "none"}')
