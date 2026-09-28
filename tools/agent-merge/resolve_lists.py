"""Resolve append-only list conflicts: keep both sides, recount `; N] = [` arrays."""
import re, sys
path = sys.argv[1]
lines = open(path).read().split("\n")
out, i = [], 0
while i < len(lines):
    if lines[i].startswith("<<<<<<< "):
        ours, theirs, j, part = [], [], i + 1, "ours"
        while not lines[j].startswith(">>>>>>> "):
            l = lines[j]
            if l.startswith("||||||| "): part = "base"
            elif l.startswith("======="): part = "theirs"
            elif part == "ours": ours.append(l)
            elif part == "theirs": theirs.append(l)
            j += 1
        if len(ours) == 1 and len(theirs) == 1 and re.search(r"; \d+\] = \[", ours[0]):
            out.append(ours[0])
        else:
            out += ours + [l for l in theirs if l not in ours or not l.strip().startswith("(")]
        i = j + 1
    else:
        out.append(lines[i]); i += 1
# recount arrays
res, k = [], 0
while k < len(out):
    m = re.match(r"^(\s*(?:pub )?const \w+: \[.*; )(\d+)(\] = \[)\s*$", out[k])
    if m:
        n, depth, j = 0, 0, k + 1
        while not re.match(r"^\s*\];", out[j]):
            s = out[j].strip()
            if depth == 0 and s.startswith("("): n += 1
            depth += s.count("(") - s.count(")") if not s.startswith("//") else 0
            j += 1
        res.append(f"{m.group(1)}{n}{m.group(3)}")
    else:
        res.append(out[k])
    k += 1
open(path, "w").write("\n".join(res))
assert not any(l.startswith(("<<<<<<<", ">>>>>>>")) for l in res)
