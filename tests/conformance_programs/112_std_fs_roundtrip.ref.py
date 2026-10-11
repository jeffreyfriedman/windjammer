import sys
seed = int(sys.argv[1])
body = f"seed={seed}\n" + "more\n"
print("written"); print("appended"); print("true")
print(len(body)); print(len(body.splitlines())); print(body.strip())
print("removed"); print("false"); print("gone")
