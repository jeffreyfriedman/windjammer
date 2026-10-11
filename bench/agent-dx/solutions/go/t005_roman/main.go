package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

var vals = []int{1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1}
var syms = []string{"M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"}

func toRoman(n int) string {
	var sb strings.Builder
	for i, v := range vals {
		for n >= v {
			sb.WriteString(syms[i])
			n -= v
		}
	}
	return sb.String()
}

func fromRoman(s string) int {
	m := map[byte]int{'I': 1, 'V': 5, 'X': 10, 'L': 50, 'C': 100, 'D': 500, 'M': 1000}
	total := 0
	for i := 0; i < len(s); i++ {
		v := m[s[i]]
		if i+1 < len(s) && v < m[s[i+1]] {
			total -= v
		} else {
			total += v
		}
	}
	return total
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" {
			continue
		}
		if n, err := strconv.Atoi(line); err == nil {
			fmt.Println(toRoman(n))
		} else {
			fmt.Println(fromRoman(line))
		}
	}
}
