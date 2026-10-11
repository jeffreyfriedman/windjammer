package main

import (
	"bufio"
	"fmt"
	"os"
	"sync"
)

func divisors(n int64) int {
	count := 0
	for d := int64(1); d*d <= n; d++ {
		if n%d == 0 {
			count++
			if d*d != n {
				count++
			}
		}
	}
	return count
}

func main() {
	r := bufio.NewReader(os.Stdin)
	var w, n int
	fmt.Fscan(r, &w, &n)
	vals := make([]int64, n)
	for i := range vals {
		fmt.Fscan(r, &vals[i])
	}
	results := make([]int, n)
	jobs := make(chan int)
	var wg sync.WaitGroup
	for i := 0; i < w; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for j := range jobs {
				results[j] = divisors(vals[j])
			}
		}()
	}
	for i := range vals {
		jobs <- i
	}
	close(jobs)
	wg.Wait()
	for i, v := range vals {
		fmt.Println(v, results[i])
	}
}
