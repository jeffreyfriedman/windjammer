package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

// Memo caches results of a pure function and counts hits and misses.
type Memo struct {
	fn           func(int64) int64
	cache        map[int64]int64
	hits, misses int
}

func NewMemo(fn func(int64) int64) *Memo {
	return &Memo{fn: fn, cache: map[int64]int64{}}
}

func (m *Memo) Get(arg int64) (int64, bool) {
	if v, ok := m.cache[arg]; ok {
		m.hits++
		return v, true
	}
	m.misses++
	v := m.fn(arg)
	m.cache[arg] = v
	return v, false
}

func fib(n int64) int64 {
	const mod = 1000000007
	a, b := int64(0), int64(1)
	for i := int64(0); i < n; i++ {
		a, b = b, (a+b)%mod
	}
	return a
}

func collatz(n int64) int64 {
	var steps int64
	for ; n != 1; steps++ {
		if n%2 == 0 {
			n /= 2
		} else {
			n = 3*n + 1
		}
	}
	return steps
}

func tri(n int64) int64 { return n * (n + 1) / 2 }

func main() {
	memos := map[string]*Memo{"fib": NewMemo(fib), "collatz": NewMemo(collatz), "tri": NewMemo(tri)}
	sc := bufio.NewScanner(os.Stdin)
	hits, misses := 0, 0
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		m, ok := memos[f[0]]
		if !ok {
			fmt.Println("unknown function")
			continue
		}
		arg, _ := strconv.ParseInt(f[1], 10, 64)
		v, hit := m.Get(arg)
		if hit {
			hits++
			fmt.Println(v, "hit")
		} else {
			misses++
			fmt.Println(v, "miss")
		}
	}
	fmt.Printf("hits=%d misses=%d\n", hits, misses)
}
