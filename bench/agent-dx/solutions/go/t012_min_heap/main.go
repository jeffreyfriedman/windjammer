package main

import (
	"bufio"
	"container/heap"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type item struct {
	prio  int64
	seq   int
	label string
}

type pq []item

func (h pq) Len() int { return len(h) }
func (h pq) Less(i, j int) bool {
	if h[i].prio != h[j].prio {
		return h[i].prio < h[j].prio
	}
	return h[i].seq < h[j].seq
}
func (h pq) Swap(i, j int) { h[i], h[j] = h[j], h[i] }
func (h *pq) Push(x any)   { *h = append(*h, x.(item)) }
func (h *pq) Pop() any {
	old := *h
	x := old[len(old)-1]
	*h = old[:len(old)-1]
	return x
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	h := &pq{}
	seq := 0
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		switch f[0] {
		case "push":
			p, _ := strconv.ParseInt(f[1], 10, 64)
			heap.Push(h, item{p, seq, f[2]})
			seq++
		case "pop", "peek":
			if h.Len() == 0 {
				fmt.Println("empty")
				continue
			}
			it := (*h)[0]
			if f[0] == "pop" {
				heap.Pop(h)
			}
			fmt.Println(it.prio, it.label)
		case "size":
			fmt.Println(h.Len())
		}
	}
}
