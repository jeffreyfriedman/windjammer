package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"sort"
	"strconv"
	"strings"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	first, _ := r.ReadString('\n')
	k, _ := strconv.Atoi(strings.TrimSpace(first))
	data, _ := io.ReadAll(r)
	counts := map[string]int{}
	var cur []byte
	flush := func() {
		if len(cur) > 0 {
			counts[string(cur)]++
			cur = cur[:0]
		}
	}
	for _, c := range data {
		switch {
		case c >= 'A' && c <= 'Z':
			cur = append(cur, c+32)
		case c >= 'a' && c <= 'z', c >= '0' && c <= '9':
			cur = append(cur, c)
		default:
			flush()
		}
	}
	flush()
	words := make([]string, 0, len(counts))
	for w := range counts {
		words = append(words, w)
	}
	sort.Slice(words, func(i, j int) bool {
		if counts[words[i]] != counts[words[j]] {
			return counts[words[i]] > counts[words[j]]
		}
		return words[i] < words[j]
	})
	for i := 0; i < len(words) && i < k; i++ {
		fmt.Println(words[i], counts[words[i]])
	}
}
