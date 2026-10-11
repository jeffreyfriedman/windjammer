package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type slot struct{ price, units int }

func change(amount int) string {
	var coins []string
	for _, c := range []int{100, 25, 10, 5} {
		for amount >= c {
			coins = append(coins, strconv.Itoa(c))
			amount -= c
		}
	}
	if len(coins) == 0 {
		return "none"
	}
	return strings.Join(coins, " ")
}

func main() {
	stock := map[string]*slot{"cola": {150, 3}, "chips": {100, 2}, "gum": {35, 5}}
	credit := 0
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		switch f[0] {
		case "coin":
			v, _ := strconv.Atoi(f[1])
			if v == 5 || v == 10 || v == 25 || v == 100 {
				credit += v
				fmt.Println("credit", credit)
			} else {
				fmt.Println("rejected", f[1])
			}
		case "select":
			s, ok := stock[f[1]]
			switch {
			case !ok:
				fmt.Println("error: unknown item", f[1])
			case s.units == 0:
				fmt.Println("error: sold out", f[1])
			case credit < s.price:
				fmt.Printf("error: need %d more\n", s.price-credit)
			default:
				s.units--
				fmt.Printf("dispensed %s change: %s\n", f[1], change(credit-s.price))
				credit = 0
			}
		case "refund":
			fmt.Println("refund:", change(credit))
			credit = 0
		case "status":
			fmt.Printf("credit=%d cola=%d chips=%d gum=%d\n", credit,
				stock["cola"].units, stock["chips"].units, stock["gum"].units)
		}
	}
}
