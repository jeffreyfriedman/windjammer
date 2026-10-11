import os

dirs = files = 0


def walk(path, depth):
    global dirs, files
    entries = sorted(os.listdir(path), key=os.fsencode)
    for e in entries:
        full = os.path.join(path, e)
        if os.path.isdir(full):
            dirs += 1
            print("  " * depth + e + "/")
            walk(full, depth + 1)
        else:
            files += 1
            print("  " * depth + "%s (%d bytes)" % (e, os.path.getsize(full)))


walk(".", 0)
print("%d directories, %d files" % (dirs, files))
