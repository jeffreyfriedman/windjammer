package main

import (
	"bufio"
	"fmt"
	"iter"
	"os"
	"strconv"
	"strings"
)

func fib() iter.Seq[int64] {
	return func(yield func(int64) bool) {
		a, b := int64(0), int64(1)
		for yield(a) {
			a, b = b, a+b
		}
	}
}

func squares() iter.Seq[int64] {
	return func(yield func(int64) bool) {
		for i := int64(1); yield(i * i); i++ {
		}
	}
}

func evens() iter.Seq[int64] {
	return func(yield func(int64) bool) {
		for i := int64(0); yield(i); i += 2 {
		}
	}
}

func triangular() iter.Seq[int64] {
	return func(yield func(int64) bool) {
		t := int64(0)
		for i := int64(1); ; i++ {
			t += i
			if !yield(t) {
				return
			}
		}
	}
}

func primes() iter.Seq[int64] {
	return func(yield func(int64) bool) {
		for n := int64(2); ; n++ {
			isPrime := true
			for d := int64(2); d*d <= n; d++ {
				if n%d == 0 {
					isPrime = false
					break
				}
			}
			if isPrime && !yield(n) {
				return
			}
		}
	}
}

func main() {
	seqs := map[string]func() iter.Seq[int64]{
		"fib": fib, "squares": squares, "primes": primes, "evens": evens, "triangular": triangular,
	}
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) != 3 {
			continue
		}
		n, _ := strconv.ParseInt(f[2], 10, 64)
		var items []string
		var sum, count int64
		for v := range seqs[f[0]]() {
			if f[1] == "below" && v >= n || f[1] != "below" && count >= n {
				break
			}
			count++
			sum += v
			items = append(items, strconv.FormatInt(v, 10))
		}
		if f[1] == "sum" {
			fmt.Println(sum)
		} else {
			fmt.Println(strings.Join(items, " "))
		}
	}
}
