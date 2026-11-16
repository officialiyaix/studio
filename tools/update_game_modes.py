#!/usr/bin/env python3
"""Adds game modes to launcher/src-tauri/stack/stacks.json: named sets of extra mods a Save can switch between.

For each supported Minecraft version and each game mode's mods this script:
  1. picks the pinned Modrinth version if one is listed for that Minecraft version, otherwise Modrinth's newest
     *release* Fabric build listed for it (a beta only if no release exists), skipping builds their author has
     named "[obsolete] …", and leaving a project out of a version whose newest build is marked obsolete,
  2. downloads it and checks the SHA-512 Modrinth publishes,
  3. reads the jar's fabric.mod.json and confirms its `depends.minecraft` range includes the version and that
     every mod it depends on is there in the version range it asks for (falling back to an older build if not),
  4. adds the libraries it requires (Modrinth's required dependencies, checked against the jar's own `depends`),
     unless the Stabled stack already provides them,
and leaves out a mod whose requirements can't all be met, recording why. The pinned performance stack in
stacks.json is not changed. Files are downloaded on the player's computer from Modrinth and verified by SHA-512;
Stabled does not redistribute them.

    python3 tools/update_game_modes.py        # needs network access to Modrinth (uses .stack-cache)
    python3 tools/update_game_modes.py 1.20   # only these versions; the others are kept
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import update_stacks as us  # noqa: E402

GAME_MODES = [
    {
        'id': 'cpvp',
        'name': 'CPVP',
        'description': 'Crystal PvP: PvP tiers on name tags, shield and totem tweaks, and an in-game account switcher.',
        'mods': [
            # Modrinth slug, title, purpose
            ('tiertagger', 'Tier Tagger', "Shows players' PvP tiers (MCTiers and SubTiers) on their name tags"),
            ('shield-statuses', 'Shield Statuses', 'Shows shield states by colour'),
            ('complete-shield-fixes', 'Shield Fixes', 'Fixes shield sounds and the blocking animation'),
            ('cpvp', 'Totem Tweaks', 'Smaller totem pops and a customisable pop animation'),
            ('in-game-account-switcher', 'In-Game Account Switcher', 'Change the logged-in account without restarting Minecraft'),
        ],
        # The exact builds chosen for these versions (Modrinth version ids).
        'pins': {
            '1.21.11': {
                'tiertagger': 'S6YDELz7',
                'shield-statuses': 'gFgqm4Vo',
                'complete-shield-fixes': 'U0KdeHqo',
                'cpvp': 'C2xuYDRu',
                'in-game-account-switcher': 'MwO8YI2m',
            },
        },
    },
    {
        'id': 'standard',
        'name': 'Standard',
        'description': 'Only the Stabled client and its performance mods.',
        'mods': [],
        'pins': {},
    },
]
DEFAULT_MODE = 'cpvp'
RANK = {'release': 0, 'beta': 1, 'alpha': 2}


def loader_for(mc):
    """Modrinth's loader tag for a Minecraft version: Legacy Fabric for 1.8.9 and 1.9.4."""
    return 'legacy-fabric' if mc in us.LEGACY else 'fabric'


def versions_of(project, loader='fabric', cache={}):
    key = (project, loader)
    if key not in cache:
        cache[key] = us.get_json(f'https://api.modrinth.com/v2/project/{project}/version?loaders=%5B%22{loader}%22%5D')
    return cache[key]


def project_info(project, cache={}):
    if project not in cache:
        cache[project] = us.get_json(f'https://api.modrinth.com/v2/project/{project}')
    return cache[project]


def pick(project, mc, pinned=None, have=None):
    """The build of `project` for `mc`: (version, file, fabric.mod.json, provided ids) or (None, reason).

    With `have` ({mod id: version}, what the stack already provides), builds that ask for a different version of
    one of those mods are skipped, so an older build that fits is chosen instead."""
    loader = loader_for(mc)
    vs = versions_of(project, loader)

    def obsolete(v):
        return v.get('name', '').strip().lower().startswith('[obsolete')

    # When the newest build for this version is marked obsolete, the author no longer supports the mod there
    # (Tier Tagger then warns "this version is likely broken" at every start), so it's left out.
    for_mc = sorted((v for v in vs if mc in v['game_versions'] and loader in v['loaders']), key=lambda v: v['date_published'])
    if for_mc and obsolete(for_mc[-1]) and not pinned:
        return None, f'its author marks it obsolete on Minecraft {mc}'
    cands = [v for v in vs if mc in v['game_versions'] and loader in v['loaders'] and v['version_type'] != 'alpha' and not obsolete(v)]
    if pinned:
        cands = [v for v in vs if v['id'] == pinned] or cands
    cands.sort(key=lambda v: (RANK[v['version_type']], [-ord(c) for c in v['date_published']]))
    for v in cands:
        f = next((x for x in v['files'] if x['primary']), v['files'][0])
        data = us.get_bytes(f['url'], f['hashes']['sha512'])
        j, provided = us.mod_meta(data)
        pred = (j.get('depends') or {}).get('minecraft')
        if pred is not None and not us.matches(pred, mc):
            continue
        if have and any(d in have and have[d] and not us.matches(p, have[d]) for d, p in (j.get('depends') or {}).items()):
            continue
        return v, f, j, provided
    return None, f"no {'Legacy Fabric' if loader == 'legacy-fabric' else 'Fabric'} build for Minecraft {mc}"


def entry(v, f, j, title, purpose, library=False, needed_by=None):
    return {
        'modId': j['id'], 'title': title, 'purpose': purpose, 'required': False,
        'project': v['project_id'], 'versionId': v['id'], 'versionNumber': v['version_number'],
        'versionType': v['version_type'], 'filename': f['filename'], 'url': f['url'],
        'sha512': f['hashes']['sha512'], 'size': f['size'], 'license': None, 'modVersion': j.get('version'),
        'library': library, 'neededBy': needed_by or [],
    }


def stack_provides(stack):
    """{mod id: version} the pinned stack already provides (with Fabric API's modules), plus the Stabled client,
    Minecraft and the loader."""
    provided = {i: '' for i in us.BUILTIN} | {'stabled': '', 'minecraft': stack['minecraft'],
                                              'fabricloader': stack['loader']['version']}
    for m in stack['mods']:
        provided.update(us.mod_versions(us.get_bytes(m['url'], m['sha512'])))
    return provided


def unmet(depends, have, mc):
    """The dependencies in a fabric.mod.json `depends` map that `have` ({id: version}) doesn't satisfy, as text."""
    out = []
    for dep, pred in (depends or {}).items():
        if dep.startswith('java'):
            continue
        if dep not in have:
            out.append(f"{dep}, which isn't available for Minecraft {mc}")
        elif have[dep] and not us.matches(pred, have[dep]):
            out.append(f'{dep} {pred} (Minecraft {mc} has {have[dep]})')
    return out


def resolve(mode, stack):
    mc = stack['minecraft']
    pins = mode['pins'].get(mc, {})
    base = stack_provides(stack)
    chosen, notes = [], []
    libs = {}  # project id -> (entry, {mod id: version})
    provided = dict(base)
    for slug, title, purpose in mode['mods']:
        got = pick(slug, mc, pins.get(slug), base)
        if got[0] is None:
            notes.append(f'{title}: {got[1]}')
            continue
        v, f, j, _ = got
        prov = us.mod_versions(us.get_bytes(f['url'], f['hashes']['sha512']))
        # Libraries it needs: Modrinth's required dependencies, then anything its fabric.mod.json still lacks.
        needed, ok, reason = [], True, ''
        for dep in v['dependencies']:
            if dep['dependency_type'] != 'required' or not dep.get('project_id'):
                continue
            pid = dep['project_id']
            if pid in libs:
                needed.append(pid)
                continue
            dgot = pick(pid, mc, None, base)
            if dgot[0] is None:
                ok, reason = False, f"needs {project_info(pid)['title']}, which has no Fabric build for Minecraft {mc}"
                break
            dv, df, dj, _ = dgot
            dprov = us.mod_versions(us.get_bytes(df['url'], df['hashes']['sha512']))
            if dj['id'] in base:
                continue  # the Stabled stack already has it (e.g. Fabric API)
            libs[pid] = (entry(dv, df, dj, project_info(pid)['title'], 'Library', True), dprov)
            needed.append(pid)
        if ok:
            # Every `depends` entry must be present *and* in the version range it asks for (e.g. In-Game
            # Account Switcher for 1.20 asks for a newer Fabric API than 1.20 has).
            have = {**provided, **prov}
            for p in needed:
                have.update(libs[p][1])
            missing = unmet(j.get('depends'), have, mc)
            if missing:
                ok, reason = False, 'needs ' + '; '.join(missing)
        if not ok:
            notes.append(f'{title}: {reason}')
            continue
        for p in needed:
            e, dprov = libs[p]
            if title not in e['neededBy']:
                e['neededBy'].append(title)
            provided.update(dprov)
        provided.update(prov)
        chosen.append(entry(v, f, j, title, purpose))
    # Libraries no chosen mod ended up needing are dropped.
    used = [e for e, _ in libs.values() if e['neededBy']]
    for e in used:
        e['purpose'] = 'Library needed by ' + ' and '.join(e['neededBy'])
    # Declared incompatibilities inside the mode, and with the stack.
    all_mods = chosen + used
    metas = {}
    for m in all_mods + stack['mods']:
        j, _ = us.mod_meta(us.get_bytes(m['url'], m['sha512']))
        metas[j['id']] = j
    for m in all_mods:
        j = metas[m['modId']]
        for other, pred in (j.get('breaks') or {}).items():
            if other in metas and us.matches(pred, metas[other].get('version') or ''):
                notes.append(f"{m['title']} declares it breaks {other}")
    return chosen + used, notes


def main():
    path = us.OUT
    data = json.load(open(path))
    # `python3 tools/update_game_modes.py 1.20 1.8.9` resolves only those versions and keeps the others.
    only = [a for a in sys.argv[1:] if not a.startswith('-')]
    for stack in data['stacks']:
        if only and stack['minecraft'] not in only:
            continue
        stack['modes'] = {}
        for mode in GAME_MODES:
            mods, notes = resolve(mode, stack)
            stack['modes'][mode['id']] = {'mods': mods, 'notes': notes}
            if mode['mods']:
                main_titles = [m['title'] + ' ' + m['versionNumber'] for m in mods if not m['library']]
                lib_titles = [m['title'] for m in mods if m['library']]
                print(stack['minecraft'].ljust(8), mode['name'], '|', ', '.join(main_titles), '| libraries:', ', '.join(lib_titles) or '-')
                for n in notes:
                    print('         -', n)
    data['gameModes'] = [{'id': m['id'], 'name': m['name'], 'description': m['description']} for m in GAME_MODES]
    data['defaultGameMode'] = DEFAULT_MODE
    with open(path, 'w') as fh:
        json.dump(data, fh, indent=1)
    print('wrote', path)


if __name__ == '__main__':
    sys.exit(main())
