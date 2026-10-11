package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type Expr interface {
	Eval() int64
	Depth() int
	Nodes() int
	Render() string
}

type Lit int64
type Neg struct{ X Expr }
type Bin struct {
	Op   byte
	L, R Expr
}

func (l Lit) Eval() int64    { return int64(l) }
func (l Lit) Depth() int     { return 1 }
func (l Lit) Nodes() int     { return 1 }
func (l Lit) Render() string { return strconv.FormatInt(int64(l), 10) }

func (n Neg) Eval() int64    { return -n.X.Eval() }
func (n Neg) Depth() int     { return 1 + n.X.Depth() }
func (n Neg) Nodes() int     { return 1 + n.X.Nodes() }
func (n Neg) Render() string { return "-(" + n.X.Render() + ")" }

func (b Bin) Eval() int64 {
	l, r := b.L.Eval(), b.R.Eval()
	switch b.Op {
	case '+':
		return l + r
	case '-':
		return l - r
	}
	return l * r
}
func (b Bin) Depth() int { return 1 + max(b.L.Depth(), b.R.Depth()) }
func (b Bin) Nodes() int { return 1 + b.L.Nodes() + b.R.Nodes() }
func (b Bin) Render() string {
	return "(" + b.L.Render() + " " + string(b.Op) + " " + b.R.Render() + ")"
}

type parser struct {
	toks []string
	pos  int
}

func tokenize(s string) []string {
	s = strings.ReplaceAll(s, "(", " ( ")
	s = strings.ReplaceAll(s, ")", " ) ")
	return strings.Fields(s)
}

func (p *parser) next() string {
	t := p.toks[p.pos]
	p.pos++
	return t
}

func (p *parser) parse() Expr {
	t := p.next()
	if t != "(" {
		v, _ := strconv.ParseInt(t, 10, 64)
		return Lit(v)
	}
	op := p.next()
	var e Expr
	if op == "neg" {
		e = Neg{p.parse()}
	} else {
		l := p.parse()
		e = Bin{op[0], l, p.parse()}
	}
	p.next() // closing paren
	return e
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		toks := tokenize(sc.Text())
		if len(toks) == 0 {
			continue
		}
		e := (&parser{toks: toks}).parse()
		fmt.Println(e.Eval(), e.Depth(), e.Nodes(), e.Render())
	}
}
