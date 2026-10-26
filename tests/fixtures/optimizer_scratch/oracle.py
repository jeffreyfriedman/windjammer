# Independent oracle for s01_scratch_reuse.rs (same arithmetic, written separately).
import sys
for seed in [1,2,3,4,5]:
    total=0; out=[]
    for i in range(5+seed):
        line="".join(f"{(i*j+seed)%10}-" for j in range(i%4+1))+"x"
        total+=len(line)
        parts=sorted((k*seed+i)%7 for k in range(i%3+2))
        total+=sum(parts)*parts[0]
        out.append(line)
    kept=[str(i+seed) for i in range(3)]
    print(f"{total} {','.join(out)} {'|'.join(kept)}")
