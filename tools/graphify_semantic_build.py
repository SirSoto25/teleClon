import json, glob, sys
from pathlib import Path
from graphify.cache import save_semantic_cache
from graphify.extract import collect_files, extract
from graphify.build import build_from_json
from graphify.cluster import cluster, score_all
from graphify.analyze import god_nodes, surprising_connections, suggest_questions
from graphify.report import generate
from graphify.export import to_json
from graphify.diagnostics import diagnose_extraction, format_diagnostic_report

ROOT = '.'
SPEC = str(Path('.claude/skills/graphify/references/extraction-spec.md').resolve())
out = Path('graphify-out')

# B3: merge chunks
chunks = sorted(glob.glob('graphify-out/.graphify_chunk_*.json'))
nodes, edges, hyper = [], [], []
partial = []
for c in chunks:
    d = json.loads(Path(c).read_text(encoding='utf-8'))
    nodes += d.get('nodes', []); edges += d.get('edges', []); hyper += d.get('hyperedges', [])
    for src, n in (d.get('coverage') or {}).items():
        p = Path(src)
        if p.is_file():
            total = sum(1 for _ in p.open(encoding='utf-8', errors='replace'))
            if total and n < total:
                partial.append(src)
print('chunks', len(chunks), 'nodes', len(nodes), 'edges', len(edges), 'partial', partial)
seen = set(); dn = []
for n in nodes:
    if n['id'] not in seen:
        seen.add(n['id']); dn.append(n)
uncached = [l for l in (out / '.graphify_uncached.txt').read_text(encoding='utf-8').splitlines() if l]
saved = save_semantic_cache(dn, edges, hyper, root=ROOT, allowed_source_files=uncached, prompt_file=SPEC, partial_source_files=partial)
print('cached', saved)
sem = {'nodes': dn, 'edges': edges, 'hyperedges': hyper, 'input_tokens': 0, 'output_tokens': 0}
for n in sem['nodes']: n.setdefault('_origin', 'semantic')
for e in sem['edges']: e.setdefault('_origin', 'semantic')

# Part A: AST
detect = json.loads((out / '.graphify_detect.json').read_text(encoding='utf-8'))
code = []
for f in detect['files'].get('code', []):
    code.extend(collect_files(Path(f)) if Path(f).is_dir() else [Path(f)])
ast = extract(code, cache_root=Path(ROOT), root=Path(ROOT))
print('AST', len(ast['nodes']), len(ast['edges']))

# Part C
ids = {n['id'] for n in ast['nodes']}
mn = list(ast['nodes'])
for n in sem['nodes']:
    if n['id'] not in ids:
        mn.append(n); ids.add(n['id'])
merged = {'nodes': mn, 'edges': ast['edges'] + sem['edges'], 'hyperedges': sem['hyperedges'], 'input_tokens': 0, 'output_tokens': 0}
(out / '.graphify_extract.json').write_text(json.dumps(merged, ensure_ascii=False), encoding='utf-8')

# Step 4
G = build_from_json(merged, root=ROOT, directed=False)
if G.number_of_nodes() == 0:
    sys.exit('empty graph')
comms = cluster(G)
coh = score_all(G, comms)
gods = god_nodes(G)
surp = surprising_connections(G, comms)
labels = {c: 'Community ' + str(c) for c in comms}
qs = suggest_questions(G, comms, labels)
wrote = to_json(G, comms, 'graphify-out/graph.json', force=True)
rep = generate(G, comms, coh, labels, gods, surp, detect, {'input': 0, 'output': 0}, ROOT, suggested_questions=qs)
(out / 'GRAPH_REPORT.md').write_text(rep, encoding='utf-8')
(out / '.graphify_analysis.json').write_text(json.dumps({'communities': {str(k): v for k, v in comms.items()}, 'cohesion': {str(k): v for k, v in coh.items()}, 'gods': gods, 'surprises': surp, 'questions': qs}, ensure_ascii=False, indent=1), encoding='utf-8')
print('Graph', G.number_of_nodes(), G.number_of_edges(), len(comms), 'wrote', wrote)
print(format_diagnostic_report(diagnose_extraction(merged, directed=False, root=ROOT)))
