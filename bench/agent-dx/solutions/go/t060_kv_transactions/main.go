package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

// undo records the previous state of a key so a transaction can be rolled back.
type undo struct {
	key, old string
	existed  bool
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	data := map[string]string{}
	var txs [][]undo
	record := func(k string) {
		if len(txs) == 0 {
			return
		}
		old, ok := data[k]
		txs[len(txs)-1] = append(txs[len(txs)-1], undo{k, old, ok})
	}
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		switch f[0] {
		case "set":
			record(f[1])
			data[f[1]] = f[2]
		case "get":
			if v, ok := data[f[1]]; ok {
				fmt.Println(v)
			} else {
				fmt.Println("NULL")
			}
		case "delete":
			record(f[1])
			delete(data, f[1])
		case "count":
			n := 0
			for _, v := range data {
				if v == f[1] {
					n++
				}
			}
			fmt.Println(n)
		case "begin":
			txs = append(txs, nil)
		case "rollback":
			if len(txs) == 0 {
				fmt.Println("NO TRANSACTION")
				continue
			}
			log := txs[len(txs)-1]
			txs = txs[:len(txs)-1]
			for i := len(log) - 1; i >= 0; i-- {
				if log[i].existed {
					data[log[i].key] = log[i].old
				} else {
					delete(data, log[i].key)
				}
			}
		case "commit":
			if len(txs) == 0 {
				fmt.Println("NO TRANSACTION")
			}
			txs = nil
		}
	}
}
