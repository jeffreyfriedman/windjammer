package main

import (
	"bufio"
	"fmt"
	"os"
)

type result struct {
	idx int
	sum int64
}

func main() {
	r := bufio.NewReader(os.Stdin)
	var w, n int
	fmt.Fscan(r, &w, &n)
	vals := make([]int64, n)
	for i := range vals {
		fmt.Fscan(r, &vals[i])
	}
	q, rem := n/w, n%w
	ch := make(chan result)
	start := 0
	for i := 0; i < w; i++ {
		size := q
		if i < rem {
			size++
		}
		chunk := vals[start : start+size]
		start += size
		go func() {
			var s int64
			for _, v := range chunk {
				s += v
			}
			ch <- result{i, s}
		}()
	}
	sums := make([]int64, w)
	for i := 0; i < w; i++ {
		res := <-ch
		sums[res.idx] = res.sum
	}
	var total int64
	for i, s := range sums {
		fmt.Printf("chunk %d: %d\n", i, s)
		total += s
	}
	fmt.Printf("total: %d\n", total)
}
