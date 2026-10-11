package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type stage func(in <-chan int64) <-chan int64

func filter(pred func(int64) bool) stage {
	return func(in <-chan int64) <-chan int64 {
		out := make(chan int64)
		go func() {
			defer close(out)
			for x := range in {
				if pred(x) {
					out <- x
				}
			}
		}()
		return out
	}
}

func mapper(f func(int64) int64) stage {
	return func(in <-chan int64) <-chan int64 {
		out := make(chan int64)
		go func() {
			defer close(out)
			for x := range in {
				out <- f(x)
			}
		}()
		return out
	}
}

func dedupe3(in <-chan int64) <-chan int64 {
	out := make(chan int64)
	go func() {
		defer close(out)
		var prev []int64
		for x := range in {
			dup := len(prev) > 0 && prev[0] == x || len(prev) > 1 && prev[1] == x
			prev = append(prev, x)
			if len(prev) > 2 {
				prev = prev[1:]
			}
			// keep prev as [older, newer] of the last two items seen
			if !dup {
				out <- x
			}
		}
	}()
	return out
}

func parseStage(tok string) stage {
	name, arg, _ := strings.Cut(tok, ":")
	k, _ := strconv.ParseInt(arg, 10, 64)
	switch name {
	case "add":
		return mapper(func(x int64) int64 { return x + k })
	case "mul":
		return mapper(func(x int64) int64 { return x * k })
	case "gt":
		return filter(func(x int64) bool { return x > k })
	case "even":
		return filter(func(x int64) bool { return x%2 == 0 })
	case "odd":
		return filter(func(x int64) bool { return x%2 != 0 })
	case "dedupe3":
		return dedupe3
	}
	return nil
}

func main() {
	r := bufio.NewReader(os.Stdin)
	line, _ := r.ReadString('\n')
	n, _ := strconv.Atoi(strings.TrimSpace(line))
	line, _ = r.ReadString('\n')
	src := make(chan int64)
	go func() {
		defer close(src)
		for i := 1; i <= n; i++ {
			src <- int64(i)
		}
	}()
	var out <-chan int64 = src
	for _, tok := range strings.Fields(line) {
		out = parseStage(tok)(out)
	}
	var items []string
	var sum int64
	for x := range out {
		items = append(items, strconv.FormatInt(x, 10))
		sum += x
	}
	fmt.Println(len(items))
	fmt.Println(strings.Join(items, " "))
	fmt.Printf("sum=%d\n", sum)
}
