import sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

P = r"C:\Users\oli\Desktop\hcaptcha_research\proof_emulation\chrome_proof_env.json"
s = open(P, encoding="utf-8", errors="replace").read()
i = s.find("\u2764")
print("json hit", i)
print(repr(s[max(0, i-300):i+400]))

print("\n--- memory dump ---")
b = open(r"C:\Users\oli\Desktop\hcaptcha_research\analysis\mem827.bin", "rb").read()
j = b.find("\u2764".encode("utf-8"))
print("mem hit", j)
chunk = b[max(0, j-200):j+400]
print(repr(chunk))
