"""Motion transitions witnessed by the gated retail traces (docs/INTERACTION_MATRIX.md).

For every FD scenario named in the m4/m5 gates with a local tick trace, read
each Fox/Marth motion id at frame_end over the gated window and write
transitions.tsv (character, from, to, directed count, corpus count,
directed witnesses) and visits.tsv (states entered) to the output directory.
Same-tick motion changes merge; branches that keep the motion id are invisible.

    cd harness && uv run python interaction_matrix.py <out-dir>
"""
import json, re, os, sys, collections, tomllib
R=os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
S=sys.argv[1] if len(sys.argv)>1 else '.'
os.makedirs(S, exist_ok=True)
# names
src=open(R+'/crates/melee-types/src/motion_state.rs').read()
common={}
for m in re.finditer(r'^\s+([A-Z][A-Za-z0-9_]+) = (-?\d+)',src,re.M):
    common.setdefault(int(m.group(2)),m.group(1))
fox="SpecialNStart SpecialNLoop SpecialNEnd SpecialAirNStart SpecialAirNLoop SpecialAirNEnd SpecialSStart SpecialS SpecialSEnd SpecialAirSStart SpecialAirS SpecialAirSEnd SpecialHiHold SpecialHiHoldAir SpecialHi SpecialAirHi SpecialHiLanding SpecialHiFall SpecialHiBound SpecialLwStart SpecialLwLoop SpecialLwHit SpecialLwEnd SpecialLwTurn SpecialAirLwStart SpecialAirLwLoop SpecialAirLwHit SpecialAirLwEnd SpecialAirLwTurn".split()
mar="SpecialNStart SpecialNLoop SpecialNEnd0 SpecialNEnd1 SpecialAirNStart SpecialAirNLoop SpecialAirNEnd0 SpecialAirNEnd1 SpecialS1 SpecialS2Hi SpecialS2Lw SpecialS3Hi SpecialS3S SpecialS3Lw SpecialS4Hi SpecialS4S SpecialS4Lw SpecialAirS1 SpecialAirS2Hi SpecialAirS2Lw SpecialAirS3Hi SpecialAirS3S SpecialAirS3Lw SpecialAirS4Hi SpecialAirS4S SpecialAirS4Lw SpecialHi SpecialAirHi SpecialLw SpecialLwHit SpecialAirLw SpecialAirLwHit".split()
KIND={1:'Fox',18:'Marth'}
def name(k,m):
    if m>=341:
        t=fox if k==1 else mar
        i=m-341
        return ('Fx.' if k==1 else 'Ms.')+(t[i] if i<len(t) else str(m))
    return common.get(m,str(m))
gated=set()
for f in ['m5_gate.rs','m4_gate.rs']:
    gated|=set(re.findall(r'"([a-z0-9_]+)"',open(R+'/crates/melee-sim/tests/'+f).read()))
scen={}
for n in sorted(gated):
    p=R+'/harness/scenarios/'+n+'.toml'
    t=R+'/harness/traces/'+n+'.tick.expected.jsonl'
    if re.search(r"_(bf|dl|ys|fod)(_|$)",n) or n=="platform_bf_fox": continue
    if os.path.exists(p) and os.path.exists(t):
        scen[n]=(tomllib.load(open(p,'rb')).get('frames'),t)
trans=collections.defaultdict(lambda: collections.OrderedDict())
visit=collections.defaultdict(set)
missing=[]
for n,(frames,t) in scen.items():
    prev={}
    corpus=n.startswith('corpus')
    for i,l in enumerate(open(t)):
        if frames and i>=frames: break
        st=json.loads(l)['state']
        for p in range(4):
            k=st.get(f'p{p}.kind'); m=st.get(f'p{p}.motion_id')
            if not k or not m: continue
            k=k['v']; m=m['v']
            if k not in KIND: continue
            key=(k,p)
            visit[(k,m)].add(n)
            if key in prev and prev[key]!=m:
                trans[(k,prev[key],m)][n]=True
            prev[key]=m
out=open(S+'/transitions.tsv','w')
for (k,a,b),ns in sorted(trans.items()):
    names=list(ns)
    d=[x for x in names if not x.startswith('corpus')]
    c=[x for x in names if x.startswith('corpus')]
    out.write(f"{KIND[k]}\t{name(k,a)}\t{name(k,b)}\t{len(d)}\t{len(c)}\t{','.join(d)}\n")
out.close()
out=open(S+'/visits.tsv','w')
for (k,m),ns in sorted(visit.items()):
    d=[x for x in ns if not x.startswith('corpus')]
    out.write(f"{KIND[k]}\t{m}\t{name(k,m)}\t{len(d)}\t{len(ns)-len(d)}\t{','.join(sorted(d)[:4])}\n")
out.close()
print(len(scen),'scenarios;',len(trans),'transitions;',len(visit),'states')
