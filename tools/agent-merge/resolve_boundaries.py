"""Union both sides of a boundaries.toml conflict; every entry keeps its [[boundary]] header.
The first side keeps whatever header precedes the hunk; the second side gets its own."""
import re, sys, tomllib
p = sys.argv[1]; s = open(p).read()
def second(t):
    t = t.strip("\n")
    return t if t.lstrip().startswith("[[boundary]]") else "[[boundary]]\n" + t
s = re.sub(r"<<<<<<< [^\n]*\n(.*?)\|\|\|\|\|\|\| [^\n]*\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n",
           lambda m: m.group(1).rstrip("\n") + "\n\n" + second(m.group(3)) + "\n", s, flags=re.S)
b = tomllib.loads(s)["boundary"]
names = [x.get("name") for x in b]
assert None not in names and len(names) == len(set(names)), "bad or duplicate boundary entries"
open(p, "w").write(s)
print(len(b), "boundaries")
