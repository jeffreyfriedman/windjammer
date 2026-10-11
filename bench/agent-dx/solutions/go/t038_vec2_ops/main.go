package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type Vec2 struct{ X, Y int64 }

func (a Vec2) Add(b Vec2) Vec2    { return Vec2{a.X + b.X, a.Y + b.Y} }
func (a Vec2) Sub(b Vec2) Vec2    { return Vec2{a.X - b.X, a.Y - b.Y} }
func (a Vec2) Dot(b Vec2) int64   { return a.X*b.X + a.Y*b.Y }
func (a Vec2) Cross(b Vec2) int64 { return a.X*b.Y - a.Y*b.X }
func (a Vec2) Scale(k int64) Vec2 { return Vec2{a.X * k, a.Y * k} }
func (a Vec2) String() string     { return fmt.Sprintf("(%d, %d)", a.X, a.Y) }

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) != 5 {
			continue
		}
		n := make([]int64, 4)
		for i, idx := range []int{0, 1, 3, 4} {
			n[i], _ = strconv.ParseInt(f[idx], 10, 64)
		}
		a, b := Vec2{n[0], n[1]}, Vec2{n[2], n[3]}
		switch f[2] {
		case "+":
			fmt.Println(a.Add(b))
		case "-":
			fmt.Println(a.Sub(b))
		case "*":
			fmt.Println(a.Dot(b))
		case "x":
			fmt.Println(a.Cross(b))
		case "s":
			fmt.Println(a.Scale(b.X))
		case "==":
			fmt.Println(a == b)
		}
	}
}
