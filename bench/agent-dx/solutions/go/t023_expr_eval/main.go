package main

import (
	"bufio"
	"errors"
	"fmt"
	"os"
	"strconv"
)

var errBad = errors.New("bad")

type parser struct {
	s   string
	pos int
}

func (p *parser) skip() {
	for p.pos < len(p.s) && p.s[p.pos] == ' ' {
		p.pos++
	}
}

func (p *parser) peek() byte {
	p.skip()
	if p.pos < len(p.s) {
		return p.s[p.pos]
	}
	return 0
}

func (p *parser) expr() (int64, error) {
	v, err := p.term()
	for err == nil {
		c := p.peek()
		if c != '+' && c != '-' {
			break
		}
		p.pos++
		var r int64
		if r, err = p.term(); err != nil {
			break
		}
		if c == '+' {
			v += r
		} else {
			v -= r
		}
	}
	return v, err
}

func (p *parser) term() (int64, error) {
	v, err := p.unary()
	for err == nil {
		c := p.peek()
		if c != '*' && c != '/' && c != '%' {
			break
		}
		p.pos++
		var r int64
		if r, err = p.unary(); err != nil {
			break
		}
		switch c {
		case '*':
			v *= r
		case '/', '%':
			if r == 0 {
				return 0, errBad
			}
			if c == '/' {
				v /= r
			} else {
				v %= r
			}
		}
	}
	return v, err
}

func (p *parser) unary() (int64, error) {
	c := p.peek()
	switch {
	case c == '-':
		p.pos++
		v, err := p.unary()
		return -v, err
	case c == '(':
		p.pos++
		v, err := p.expr()
		if err != nil || p.peek() != ')' {
			return 0, errBad
		}
		p.pos++
		return v, nil
	case c >= '0' && c <= '9':
		start := p.pos
		for p.pos < len(p.s) && p.s[p.pos] >= '0' && p.s[p.pos] <= '9' {
			p.pos++
		}
		return strconv.ParseInt(p.s[start:p.pos], 10, 64)
	}
	return 0, errBad
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	for sc.Scan() {
		p := &parser{s: sc.Text()}
		v, err := p.expr()
		if err != nil || p.peek() != 0 {
			fmt.Println("error")
		} else {
			fmt.Println(v)
		}
	}
}
