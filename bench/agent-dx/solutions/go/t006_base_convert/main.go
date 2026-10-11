package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) != 3 {
			continue
		}
		from, _ := strconv.Atoi(f[1])
		to, _ := strconv.Atoi(f[2])
		v, _ := strconv.ParseUint(strings.ToLower(f[0]), from, 64)
		fmt.Println(strings.ToUpper(strconv.FormatUint(v, to)))
	}
}
