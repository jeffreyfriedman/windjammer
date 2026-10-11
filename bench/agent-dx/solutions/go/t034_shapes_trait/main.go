package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type Shape interface {
	Name() string
	Corners() int
	DoubleArea() int64
}

type shape struct {
	name    string
	corners int
	area2   int64
}

func (s shape) Name() string      { return s.name }
func (s shape) Corners() int      { return s.corners }
func (s shape) DoubleArea() int64 { return s.area2 }

func parse(f []string) Shape {
	n := make([]int64, len(f)-1)
	for i, s := range f[1:] {
		n[i], _ = strconv.ParseInt(s, 10, 64)
	}
	switch f[0] {
	case "square":
		return shape{f[0], 4, 2 * n[0] * n[0]}
	case "rect":
		return shape{f[0], 4, 2 * n[0] * n[1]}
	case "rtri":
		return shape{f[0], 3, n[0] * n[1]}
	case "trap":
		return shape{f[0], 4, (n[0] + n[1]) * n[2]}
	}
	return nil
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	var total int64
	var largest Shape
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		s := parse(f)
		if s == nil {
			continue
		}
		fmt.Printf("%s corners=%d area2=%d\n", s.Name(), s.Corners(), s.DoubleArea())
		total += s.DoubleArea()
		if largest == nil || s.DoubleArea() > largest.DoubleArea() {
			largest = s
		}
	}
	fmt.Printf("total area2=%d\n", total)
	if largest == nil {
		fmt.Println("largest=none")
	} else {
		fmt.Printf("largest=%s area2=%d\n", largest.Name(), largest.DoubleArea())
	}
}
