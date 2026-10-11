def _c(cmd, text):
    return {"stdin": f"{cmd}\n{text}"}


SAMPLES = [_c("enc 3", "Hello, World!\n"), _c("dec 3", "Khoor, Zruog!\n")]
EDGE = [_c("enc 26", "abc XYZ\n"), _c("enc -1", "aA zZ\n"), _c("enc 53", "Zebra 123\n"), _c("dec -30", "mixed\nlines\n\n"), _c("enc 0", "")]
ALPHA = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ .,!?0123456789\n"


def rand(rng):
    text = "".join(rng.choice(ALPHA) for _ in range(rng.randint(0, 60)))
    return _c(f"{rng.choice(['enc', 'dec'])} {rng.randint(-60, 60)}", text + "\n")


def solve(case):
    head, _, text = case["stdin"].partition("\n")
    cmd, n = head.split()
    n = int(n) if cmd == "enc" else -int(n)
    out = []
    for ch in text:
        if "a" <= ch <= "z":
            out.append(chr((ord(ch) - 97 + n) % 26 + 97))
        elif "A" <= ch <= "Z":
            out.append(chr((ord(ch) - 65 + n) % 26 + 65))
        else:
            out.append(ch)
    return "".join(out)
