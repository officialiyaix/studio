#!/usr/bin/env python3
"""Regenerates launcher/src-tauri/stack/stacks.json — the pinned, verified mod stack for every supported version.

For each Minecraft version and each performance mod this script:
  1. picks Modrinth's newest *release* Fabric build listed for that version (a beta only if no release exists),
  2. downloads it and checks the SHA-512 Modrinth publishes,
  3. reads the jar's own fabric.mod.json and confirms its `depends.minecraft` range really includes the version,
  4. checks every dependency is provided by the stack (or the loader) and that no `breaks` entry matches,
and drops anything that fails, recording why. Run it whenever versions are added:

    python3 tools/update_stacks.py            # needs network access to Modrinth, Mojang and Fabric
"""
import hashlib
import io
import json
import os
import re
import sys
import urllib.request
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, 'launcher', 'src-tauri', 'stack', 'stacks.json')
CACHE = os.environ.get('STABLED_STACK_CACHE', os.path.join(ROOT, '.stack-cache'))
UA = {'User-Agent': 'stabled-stack-updater (https://github.com/officialiyaix/studio)'}
LOADER = '0.20.1'

VERSIONS = ['1.20.1', '1.20.2', '1.20.3', '1.20.4', '1.20.5', '1.20.6', '1.21', '1.21.1', '1.21.2', '1.21.3', '1.21.4',
            '1.21.5', '1.21.6', '1.21.7', '1.21.8', '1.21.9', '1.21.10', '1.21.11', '26.1', '26.1.1', '26.1.2', '26.2', '26.3']

MODS = [
    # slug, title, purpose, required
    ('fabric-api', 'Fabric API', 'Required by the Stabled client', True),
    ('sodium', 'Sodium', 'Rendering performance', False),
    ('lithium', 'Lithium', 'Game logic and server-tick performance', False),
    ('ferrite-core', 'FerriteCore', 'Lower memory use', False),
    ('immediatelyfast', 'ImmediatelyFast', 'Faster HUD, text and immediate-mode rendering', False),
    ('entityculling', 'EntityCulling', 'Skips rendering hidden entities and block entities', False),
]
BUILTIN = {'minecraft', 'java', 'fabricloader', 'fabric-loader', 'mixinextras', 'fabric'}


def get_json(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
        return json.load(r)


def get_bytes(url, sha512):
    os.makedirs(CACHE, exist_ok=True)
    path = os.path.join(CACHE, sha512[:32] + '.jar')
    if os.path.exists(path):
        data = open(path, 'rb').read()
        if hashlib.sha512(data).hexdigest() == sha512:
            return data
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=120) as r:
        data = r.read()
    if hashlib.sha512(data).hexdigest() != sha512:
        raise ValueError('hash mismatch for ' + url)
    open(path, 'wb').write(data)
    return data


# ---- Fabric version predicates (same semantics as launcher/src-tauri/src/mods.rs) ----
def parse(v):
    core = v.split('+')[0]
    nums, _, pre = core.partition('-')
    return [int(p) if p.isdigit() else 0 for p in nums.split('.')], bool(pre)


def cmp(a, b):
    an, ap = parse(a)
    bn, bp = parse(b)
    for i in range(max(len(an), len(bn))):
        x = an[i] if i < len(an) else 0
        y = bn[i] if i < len(bn) else 0
        if x != y:
            return (x > y) - (x < y)
    return (bp > ap) - (bp < ap)


def constraint(c, v):
    c = c.strip()
    if not c or c == '*':
        return True
    for op in ('>=', '<=', '>', '<', '=', '~', '^'):
        if c.startswith(op):
            t = c[len(op):].rstrip('-')
            o = cmp(v, t)
            tn, _ = parse(t)
            vn, _ = parse(v)
            return {'>=': o >= 0, '<=': o <= 0, '>': o > 0, '<': o < 0, '=': o == 0,
                    '~': o >= 0 and vn[:1] == tn[:1] and (len(tn) < 2 or (vn[1] if len(vn) > 1 else 0) == tn[1]),
                    '^': o >= 0 and vn[:1] == tn[:1]}[op]
    if re.search(r'[xX*]', c):
        vn, _ = parse(v)
        for i, p in enumerate(c.split('.')):
            if p in ('x', 'X', '*'):
                return True
            if (vn[i] if i < len(vn) else 0) != int(p):
                return False
        return True
    return cmp(v, c) == 0


def matches(pred, v):
    if isinstance(pred, list):
        return any(matches(p, v) for p in pred)
    if isinstance(pred, str):
        return all(constraint(c, v) for c in pred.split())
    return True


def mod_meta(data):
    z = zipfile.ZipFile(io.BytesIO(data))
    j = json.loads(z.read('fabric.mod.json'))
    provided = {j['id'], *j.get('provides', [])}
    for n in j.get('jars', []):
        try:
            inner = json.loads(zipfile.ZipFile(io.BytesIO(z.read(n['file']))).read('fabric.mod.json'))
            provided |= {inner['id'], *inner.get('provides', [])}
        except Exception:
            pass
    return j, provided


def main():
    manifest = get_json('https://piston-meta.mojang.com/mc/game/version_manifest_v2.json')
    urls = {v['id']: v['url'] for v in manifest['versions']}
    rank = {'release': 0, 'beta': 1, 'alpha': 2}
    all_versions = {slug: get_json(f'https://api.modrinth.com/v2/project/{slug}/version?loaders=%5B%22fabric%22%5D') for slug, *_ in MODS}
    stacks = []
    for mc in VERSIONS:
        vj = get_json(urls[mc])
        fabric_profile = f'https://meta.fabricmc.net/v2/versions/loader/{mc}/{LOADER}/profile/json'
        get_json(fabric_profile)  # fails loudly if Fabric doesn't support this version
        chosen, notes, provided = [], [], set(BUILTIN)
        metas = {}
        for slug, title, purpose, required in MODS:
            cands = [v for v in all_versions[slug] if mc in v['game_versions'] and 'fabric' in v['loaders'] and v['version_type'] != 'alpha']
            cands.sort(key=lambda v: (rank[v['version_type']], v['date_published']), reverse=False)
            # Sort: releases first, newest first within each channel.
            cands.sort(key=lambda v: (rank[v['version_type']], [-ord(c) for c in v['date_published']]))
            picked = None
            for v in cands:
                f = next((x for x in v['files'] if x['primary']), v['files'][0])
                data = get_bytes(f['url'], f['hashes']['sha512'])
                j, prov = mod_meta(data)
                pred = (j.get('depends') or {}).get('minecraft')
                if pred is not None and not matches(pred, mc):
                    notes.append(f"{title} {v['version_number']} skipped: declares Minecraft {pred}")
                    continue
                picked = (v, f, j, prov)
                break
            if not picked:
                if required:
                    raise SystemExit(f'No usable {title} for {mc}')
                notes.append(f'{title}: no compatible Fabric build for {mc}')
                continue
            v, f, j, prov = picked
            provided |= prov
            metas[j['id']] = j
            chosen.append({
                'modId': j['id'], 'title': title, 'purpose': purpose, 'required': required,
                'project': v['project_id'], 'versionId': v['id'], 'versionNumber': v['version_number'],
                'versionType': v['version_type'], 'filename': f['filename'], 'url': f['url'],
                'sha512': f['hashes']['sha512'], 'size': f['size'], 'license': None,
                'modVersion': j.get('version'),
            })
        # Dependencies and declared incompatibilities inside the stack.
        provided.add('stabled')
        versions = {mid: (m.get('version') or '') for mid, m in metas.items()}
        drop = set()
        for mid, m in metas.items():
            for dep in (m.get('depends') or {}):
                if dep not in provided and not dep.startswith('java'):
                    notes.append(f'{mid} dropped: needs {dep}')
                    drop.add(mid)
            for other, pred in (m.get('breaks') or {}).items():
                if other in versions and matches(pred, versions[other]):
                    notes.append(f'{mid} dropped: breaks {other} {versions[other]}')
                    drop.add(mid)
        chosen = [c for c in chosen if c['modId'] not in drop or c['required']]
        stacks.append({
            'minecraft': mc, 'revision': 1,
            'loader': {'id': 'fabric', 'version': LOADER},
            'java': {'component': vj['javaVersion']['component'], 'majorVersion': vj['javaVersion']['majorVersion']},
            'mods': chosen, 'notes': notes,
        })
        print(mc.ljust(8), vj['javaVersion']['component'].ljust(22), ', '.join(f"{c['title']} {c['versionNumber']}{' (beta)' if c['versionType'] != 'release' else ''}" for c in chosen))
        for n in notes:
            print('         -', n)
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, 'w') as fh:
        json.dump({'$comment': 'Generated by tools/update_stacks.py. Files are downloaded from Modrinth on the user\'s computer and verified by SHA-512; Stabled does not redistribute them.',
                   'stacks': stacks}, fh, indent=1)
    print('wrote', OUT)


if __name__ == '__main__':
    sys.exit(main())
