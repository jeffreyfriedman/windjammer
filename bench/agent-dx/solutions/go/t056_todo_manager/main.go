package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type item struct {
	id   int
	text string
	done bool
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	var items []*item
	nextID := 1
	find := func(arg string) (int, *item) {
		id, _ := strconv.Atoi(arg)
		for i, it := range items {
			if it.id == id {
				return i, it
			}
		}
		return -1, nil
	}
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" {
			continue
		}
		cmd, arg, _ := strings.Cut(line, " ")
		arg = strings.TrimSpace(arg)
		switch cmd {
		case "add":
			if arg == "" {
				fmt.Println("error: missing argument")
				continue
			}
			items = append(items, &item{id: nextID, text: arg})
			fmt.Println("added", nextID)
			nextID++
		case "done", "del":
			if arg == "" {
				fmt.Println("error: missing argument")
				continue
			}
			i, it := find(arg)
			switch {
			case it == nil:
				fmt.Println("error: no such item", arg)
			case cmd == "del":
				items = append(items[:i], items[i+1:]...)
				fmt.Println("deleted", it.id)
			case it.done:
				fmt.Println("error: already done", it.id)
			default:
				it.done = true
				fmt.Println("done", it.id)
			}
		case "list":
			if arg == "" {
				arg = "all"
			}
			if arg != "all" && arg != "open" && arg != "done" {
				fmt.Println("error: bad filter", arg)
				continue
			}
			shown := 0
			for _, it := range items {
				if arg == "open" && it.done || arg == "done" && !it.done {
					continue
				}
				mark := " "
				if it.done {
					mark = "x"
				}
				fmt.Printf("%d [%s] %s\n", it.id, mark, it.text)
				shown++
			}
			if shown == 0 {
				fmt.Println("(none)")
			}
		case "clear":
			kept := items[:0]
			for _, it := range items {
				if !it.done {
					kept = append(kept, it)
				}
			}
			fmt.Println("cleared", len(items)-len(kept))
			items = kept
		case "count":
			open, done := 0, 0
			for _, it := range items {
				if it.done {
					done++
				} else {
					open++
				}
			}
			fmt.Printf("open=%d done=%d\n", open, done)
		default:
			fmt.Println("error: unknown command", cmd)
		}
	}
}
