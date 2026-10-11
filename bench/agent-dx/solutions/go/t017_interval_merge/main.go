package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
)

type iv struct{ a, b int64 }

func main() {
	r := bufio.NewReader(os.Stdin)
	var n int
	fmt.Fscan(r, &n)
	ivs := make([]iv, n)
	for i := range ivs {
		fmt.Fscan(r, &ivs[i].a, &ivs[i].b)
	}
	sort.Slice(ivs, func(i, j int) bool { return ivs[i].a < ivs[j].a })
	var out []iv
	for _, v := range ivs {
		if len(out) > 0 && v.a <= out[len(out)-1].b {
			out[len(out)-1].b = max(out[len(out)-1].b, v.b)
		} else {
			out = append(out, v)
		}
	}
	for _, v := range out {
		fmt.Println(v.a, v.b)
	}
}
