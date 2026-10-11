package main

import (
	"bufio"
	"fmt"
	"os"
	"sync"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	var w, n int
	fmt.Fscan(r, &w, &n)
	vals := make([]int, n)
	for i := range vals {
		fmt.Fscan(r, &vals[i])
	}
	var mu sync.Mutex
	var hist [10]int
	counts := make([]int, w)
	var wg sync.WaitGroup
	for id := 0; id < w; id++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for i := id; i < n; i += w {
				mu.Lock()
				hist[vals[i]]++
				mu.Unlock()
				counts[id]++
			}
		}()
	}
	wg.Wait()
	fmt.Print("hist:")
	checksum := 0
	for d, c := range hist {
		fmt.Print(" ", c)
		checksum += d * c
	}
	fmt.Println()
	for id, c := range counts {
		fmt.Printf("worker %d: %d\n", id, c)
	}
	fmt.Println("checksum:", checksum)
}
