package main

import (
	"bufio"
	"os"
	"strconv"
	"strings"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	first, _ := r.ReadString('\n')
	f := strings.Fields(first)
	n, _ := strconv.Atoi(f[1])
	if f[0] == "dec" {
		n = -n
	}
	n = ((n % 26) + 26) % 26
	shift := func(c byte, base byte) byte { return base + (c-base+byte(n))%26 }
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	for {
		b, err := r.ReadByte()
		if err != nil {
			break
		}
		switch {
		case b >= 'a' && b <= 'z':
			b = shift(b, 'a')
		case b >= 'A' && b <= 'Z':
			b = shift(b, 'A')
		}
		w.WriteByte(b)
	}
}
