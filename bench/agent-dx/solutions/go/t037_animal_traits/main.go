package main

import (
	"bufio"
	"fmt"
	"os"
	"strings"
)

type Animal interface {
	Name() string
	Kind() string
	Sound() string
	Legs() int
	Describe() string
}

type base struct {
	name, kind, sound string
	legs              int
}

func (b base) Name() string  { return b.name }
func (b base) Kind() string  { return b.kind }
func (b base) Sound() string { return b.sound }
func (b base) Legs() int     { return b.legs }

// describe is the default behaviour shared by every kind.
func describe(a Animal) string {
	return fmt.Sprintf("%s the %s says %s on %d legs", a.Name(), a.Kind(), a.Sound(), a.Legs())
}

type generic struct{ base }

func (g generic) Describe() string { return describe(g) }

type cat struct{ base }

func (c cat) Describe() string { return c.name + " the cat purrs softly" }

func newAnimal(kind, name string) Animal {
	switch kind {
	case "dog":
		return generic{base{name, kind, "woof", 4}}
	case "cat":
		return cat{base{name, kind, "meow", 4}}
	case "duck":
		return generic{base{name, kind, "quack", 2}}
	case "snake":
		return generic{base{name, kind, "hiss", 0}}
	case "spider":
		return generic{base{name, kind, "...", 8}}
	}
	return nil
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	total := 0
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		name := ""
		if len(f) > 1 {
			name = f[1]
		}
		a := newAnimal(f[0], name)
		if a == nil {
			fmt.Println("unknown kind", f[0])
			continue
		}
		fmt.Println(a.Describe())
		total += a.Legs()
	}
	fmt.Println("legs total=" + fmt.Sprint(total))
}
