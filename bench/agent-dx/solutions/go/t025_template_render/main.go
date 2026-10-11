package main

import (
	"bufio"
	"fmt"
	"io"
	"os"
	"regexp"
	"strconv"
	"strings"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	line, _ := r.ReadString('\n')
	k, _ := strconv.Atoi(strings.TrimSpace(line))
	vars := map[string]string{}
	for i := 0; i < k; i++ {
		line, _ = r.ReadString('\n')
		line = strings.TrimRight(line, "\r\n")
		key, val, _ := strings.Cut(line, "=")
		vars[key] = val
	}
	rest, _ := io.ReadAll(r)
	re := regexp.MustCompile(`\{\{ *([A-Za-z0-9_]+) *\}\}`)
	out := re.ReplaceAllStringFunc(string(rest), func(m string) string {
		name := re.FindStringSubmatch(m)[1]
		if v, ok := vars[name]; ok {
			return v
		}
		return "<missing:" + name + ">"
	})
	fmt.Print(out)
}
