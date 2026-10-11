package main

import (
	"bufio"
	"fmt"
	"math"
	"os"
	"strconv"
	"strings"
)

func eval(line string) string {
	parts := strings.Split(line, " / ")
	acc, _ := strconv.ParseInt(strings.TrimSpace(parts[0]), 10, 64)
	for i, p := range parts[1:] {
		d, _ := strconv.ParseInt(strings.TrimSpace(p), 10, 64)
		switch {
		case d == 0:
			return fmt.Sprintf("error: division by zero at step %d", i+1)
		case acc == math.MinInt64 && d == -1:
			return fmt.Sprintf("error: overflow at step %d", i+1)
		}
		acc /= d
	}
	return strconv.FormatInt(acc, 10)
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		if strings.TrimSpace(sc.Text()) == "" {
			continue
		}
		fmt.Println(eval(sc.Text()))
	}
}
