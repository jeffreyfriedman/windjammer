package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	acc := map[string]int64{}
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		fmt.Println(run(acc, f))
	}
	names := make([]string, 0, len(acc))
	for n := range acc {
		names = append(names, n)
	}
	sort.Strings(names)
	for _, n := range names {
		fmt.Println(n, acc[n])
	}
}

func run(acc map[string]int64, f []string) string {
	amountIdx := map[string]int{"open": 2, "deposit": 2, "withdraw": 2, "transfer": 3}
	switch f[0] {
	case "balance":
		if b, ok := acc[f[1]]; ok {
			return strconv.FormatInt(b, 10)
		}
		return "error: unknown account " + f[1]
	case "open", "deposit", "withdraw", "transfer":
		amt, _ := strconv.ParseInt(f[amountIdx[f[0]]], 10, 64)
		if (f[0] == "open" && amt < 0) || (f[0] != "open" && amt <= 0) {
			return "error: invalid amount"
		}
		switch f[0] {
		case "open":
			if _, ok := acc[f[1]]; ok {
				return "error: duplicate account"
			}
			acc[f[1]] = amt
		case "deposit":
			if _, ok := acc[f[1]]; !ok {
				return "error: unknown account " + f[1]
			}
			acc[f[1]] += amt
		case "withdraw":
			b, ok := acc[f[1]]
			if !ok {
				return "error: unknown account " + f[1]
			}
			if b < amt {
				return "error: insufficient funds"
			}
			acc[f[1]] -= amt
		case "transfer":
			from, to := f[1], f[2]
			for _, n := range []string{from, to} {
				if _, ok := acc[n]; !ok {
					return "error: unknown account " + n
				}
			}
			if from == to {
				return "error: same account"
			}
			if acc[from] < amt {
				return "error: insufficient funds"
			}
			acc[from] -= amt
			acc[to] += amt
		}
		return "ok"
	}
	return "error: unknown command"
}
