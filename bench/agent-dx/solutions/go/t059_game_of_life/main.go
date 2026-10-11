package main

import (
	"bufio"
	"fmt"
	"os"
)

func main() {
	r := bufio.NewReader(os.Stdin)
	var w, h, g int
	fmt.Fscan(r, &w, &h, &g)
	grid := make([][]bool, h)
	for y := range grid {
		var row string
		fmt.Fscan(r, &row)
		grid[y] = make([]bool, w)
		for x := 0; x < w; x++ {
			grid[y][x] = row[x] == '#'
		}
	}
	for ; g > 0; g-- {
		next := make([][]bool, h)
		for y := range next {
			next[y] = make([]bool, w)
			for x := range next[y] {
				n := 0
				for dy := -1; dy <= 1; dy++ {
					for dx := -1; dx <= 1; dx++ {
						if (dx != 0 || dy != 0) && grid[(y+dy+h)%h][(x+dx+w)%w] {
							n++
						}
					}
				}
				next[y][x] = n == 3 || (grid[y][x] && n == 2)
			}
		}
		grid = next
	}
	alive := 0
	for _, row := range grid {
		line := make([]byte, w)
		for x, c := range row {
			line[x] = '.'
			if c {
				line[x] = '#'
				alive++
			}
		}
		fmt.Println(string(line))
	}
	fmt.Println("alive:", alive)
}
