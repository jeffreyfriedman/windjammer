package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

func readMatrix(r *bufio.Reader) [][]int64 {
	var rows, cols int
	fmt.Fscan(r, &rows, &cols)
	m := make([][]int64, rows)
	for i := range m {
		m[i] = make([]int64, cols)
		for j := range m[i] {
			fmt.Fscan(r, &m[i][j])
		}
	}
	return m
}

func main() {
	r := bufio.NewReader(os.Stdin)
	var k int
	fmt.Fscan(r, &k)
	for ; k > 0; k-- {
		a, b := readMatrix(r), readMatrix(r)
		if len(a[0]) != len(b) {
			fmt.Println("dimension mismatch")
		} else {
			for i := range a {
				row := make([]string, len(b[0]))
				for j := range row {
					var s int64
					for x := range b {
						s += a[i][x] * b[x][j]
					}
					row[j] = fmt.Sprint(s)
				}
				fmt.Println(strings.Join(row, " "))
			}
		}
		fmt.Println("---")
	}
}
