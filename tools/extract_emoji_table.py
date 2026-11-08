import re, sys, glob, os
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

def decode(nums):
    try:
        return "".join(chr(n) for n in nums)
    except ValueError:
        return None

def is_emojiish(s):
    if not s:
        return False
    return any(0x1F000 <= ord(c) <= 0x1FAFF or 0x2600 <= ord(c) <= 0x27BF or ord(c) in (0x2764,) for c in s)

arr_re = re.compile(r"\[((?:\s*-?\d{1,7}\s*,){2,}\s*-?\d{1,7}\s*,?\s*)\]")

for p in sorted(glob.glob(r"C:\Users\oli\Desktop\hcaptcha_research\assets\hsw_*.js")):
    s = open(p, encoding="utf-8", errors="replace").read()
    found = []
    for m in arr_re.finditer(s):
        try:
            nums = [int(x) for x in m.group(1).replace(" ", "").split(",") if x]
        except ValueError:
            continue
        if not (3 <= len(nums) <= 8):
            continue
        if not all(0 < n <= 0x10FFFF for n in nums):
            continue
        st = decode(nums)
        if is_emojiish(st) and all(ord(c) >= 0x2000 for c in st):
            found.append((m.start(), nums, st))
    if found:
        print("==", os.path.basename(p), "hits", len(found))
        seen = set()
        for off, nums, st in found:
            key = tuple(nums)
            if key in seen:
                continue
            seen.add(key)
            cps = " ".join(f"U+{n:04X}" for n in nums)
            print(f"   @{off} {st!r}  {cps}")
