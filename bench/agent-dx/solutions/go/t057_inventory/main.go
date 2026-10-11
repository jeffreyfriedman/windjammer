package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
)

type sku struct{ qty, price int64 }

func num(s string) int64 {
	v, _ := strconv.ParseInt(s, 10, 64)
	return v
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	inv := map[string]*sku{}
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		switch f[0] {
		case "stock":
			qty, price := num(f[2]), num(f[3])
			if qty <= 0 {
				fmt.Println("error: bad quantity")
				continue
			}
			s, ok := inv[f[1]]
			if !ok {
				s = &sku{price: price}
				inv[f[1]] = s
			} else if s.price != price {
				fmt.Println("error: price mismatch for", f[1])
				continue
			}
			s.qty += qty
			fmt.Println("ok", s.qty)
		case "sell":
			qty := num(f[2])
			s, ok := inv[f[1]]
			switch {
			case qty <= 0:
				fmt.Println("error: bad quantity")
			case !ok:
				fmt.Println("error: unknown sku", f[1])
			case s.qty < qty:
				fmt.Printf("error: insufficient stock for %s (have %d, need %d)\n", f[1], s.qty, qty)
			default:
				s.qty -= qty
				fmt.Printf("sold %d for %d\n", qty, qty*s.price)
			}
		case "remove":
			s, ok := inv[f[1]]
			switch {
			case !ok:
				fmt.Println("error: unknown sku", f[1])
			case s.qty > 0:
				fmt.Println("error: not empty", f[1])
			default:
				delete(inv, f[1])
				fmt.Println("removed", f[1])
			}
		case "report":
			if len(inv) == 0 {
				fmt.Println("(empty)")
				continue
			}
			names := make([]string, 0, len(inv))
			for n := range inv {
				names = append(names, n)
			}
			sort.Strings(names)
			var total int64
			for _, n := range names {
				s := inv[n]
				fmt.Printf("%s %d x %d = %d\n", n, s.qty, s.price, s.qty*s.price)
				total += s.qty * s.price
			}
			fmt.Println("total value", total)
		}
	}
}
