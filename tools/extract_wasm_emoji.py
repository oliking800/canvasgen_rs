import re, sys, struct
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

P = r"C:\Users\oli\Desktop\hcaptcha_research\\analysis\\hsw_827_captured.wasm"
b = open(P, "rb").read()
print("wasm size", len(b))

# UTF-8 emoji run regex (covers misc symbols..symbols extended + ZWJ/VS/skin tones)
run = re.compile(
    "(?:[\U0001F000-\U0001FAFF\u2600-\u27BF\u2B00-\u2BFF]"
    "|[\uFE0F\u200D\U0001F3FB-\U0001F3FF])+"
)
s = b.decode("utf-8", "surrogateescape")
for m in run.finditer(s):
    a, z = m.start(), m.end()
    # context: printable run around it
    lo = max(0, a - 160)
    hi = min(len(s), z + 160)
    # split into printable ASCII context
    before = s[lo:a]
    after = s[z:hi]
    def clean(x):
        return "".join(c if (32 <= ord(c) < 127 or ord(c) in (9,)) else "." for c in x)
    print(f"\n@{a} len={z-a} {m.group()!r}")
    print("  before:", clean(before)[-160:])
    print("  after :", clean(after)[:160])

