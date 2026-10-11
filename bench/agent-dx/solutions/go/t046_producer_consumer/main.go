package main

import (
	"fmt"
	"sort"
	"sync"
)

type tally struct {
	count, chars int
	items        []string
}

func main() {
	var p, k, c int
	fmt.Scan(&p, &k, &c)
	queue := make(chan string, 4)
	var producers sync.WaitGroup
	for id := 0; id < p; id++ {
		producers.Add(1)
		go func() {
			defer producers.Done()
			for j := 1; j <= k; j++ {
				queue <- fmt.Sprintf("p%d-%d", id, j)
			}
		}()
	}
	go func() {
		producers.Wait()
		close(queue)
	}()
	tallies := make([]tally, c)
	var consumers sync.WaitGroup
	for i := 0; i < c; i++ {
		consumers.Add(1)
		go func() {
			defer consumers.Done()
			for item := range queue {
				t := &tallies[i]
				t.count++
				t.chars += len(item)
				t.items = append(t.items, item)
			}
		}()
	}
	consumers.Wait()
	var all []string
	count, chars := 0, 0
	for _, t := range tallies {
		count += t.count
		chars += t.chars
		all = append(all, t.items...)
	}
	sort.Strings(all)
	fmt.Println("items:", p*k)
	fmt.Println("chars:", chars)
	for i := 0; i < len(all) && i < 3; i++ {
		fmt.Println("first:", all[i])
	}
	fmt.Println("consumers_total:", count)
}
