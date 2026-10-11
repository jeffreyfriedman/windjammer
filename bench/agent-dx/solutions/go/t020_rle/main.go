package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func encode(s string) string {
	var sb strings.Builder
	for i := 0; i < len(s); {
		j := i
		for j < len(s) && s[j] == s[i] {
			j++
		}
		sb.WriteString(strconv.Itoa(j - i))
		sb.WriteByte(s[i])
		i = j
	}
	return sb.String()
}

func decode(s string) string {
	var sb strings.Builder
	n := 0
	for i := 0; i < len(s); i++ {
		if s[i] >= '0' && s[i] <= '9' {
			n = n*10 + int(s[i]-'0')
		} else {
			sb.WriteString(strings.Repeat(string(s[i]), n))
			n = 0
		}
	}
	return sb.String()
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		line := sc.Text()
		if line == "" {
			continue
		}
		mode, text, _ := strings.Cut(line, " ")
		if mode == "E" {
			fmt.Println(encode(text))
		} else {
			fmt.Println(decode(text))
		}
	}
}
