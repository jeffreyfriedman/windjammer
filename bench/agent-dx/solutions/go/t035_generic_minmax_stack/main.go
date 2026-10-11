package main

import (
	"bufio"
	"cmp"
	"fmt"
	"os"
	"strconv"
	"strings"
)

// Stack keeps running min and max next to each element for O(1) queries.
type Stack[T cmp.Ordered] struct {
	items, mins, maxs []T
}

func (s *Stack[T]) Push(x T) {
	lo, hi := x, x
	if n := len(s.items); n > 0 {
		lo, hi = min(lo, s.mins[n-1]), max(hi, s.maxs[n-1])
	}
	s.items = append(s.items, x)
	s.mins = append(s.mins, lo)
	s.maxs = append(s.maxs, hi)
}

func (s *Stack[T]) Pop() (T, bool) {
	var zero T
	n := len(s.items)
	if n == 0 {
		return zero, false
	}
	x := s.items[n-1]
	s.items, s.mins, s.maxs = s.items[:n-1], s.mins[:n-1], s.maxs[:n-1]
	return x, true
}

func (s *Stack[T]) Min() (T, bool) {
	var zero T
	if len(s.items) == 0 {
		return zero, false
	}
	return s.mins[len(s.mins)-1], true
}

func (s *Stack[T]) Max() (T, bool) {
	var zero T
	if len(s.items) == 0 {
		return zero, false
	}
	return s.maxs[len(s.maxs)-1], true
}

func run[T cmp.Ordered](sc *bufio.Scanner, parse func(string) T) {
	s := &Stack[T]{}
	show := func(x T, ok bool) {
		if ok {
			fmt.Println(x)
		} else {
			fmt.Println("empty")
		}
	}
	for sc.Scan() {
		line := sc.Text()
		switch {
		case line == "push" || strings.HasPrefix(line, "push "):
			s.Push(parse(strings.TrimPrefix(strings.TrimPrefix(line, "push"), " ")))
		case line == "pop":
			show(s.Pop())
		case line == "min":
			show(s.Min())
		case line == "max":
			show(s.Max())
		case line == "size":
			fmt.Println(len(s.items))
		}
	}
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Scan()
	if strings.TrimSpace(sc.Text()) == "int" {
		run(sc, func(s string) int64 { v, _ := strconv.ParseInt(strings.TrimSpace(s), 10, 64); return v })
	} else {
		run(sc, func(s string) string { return s })
	}
}
