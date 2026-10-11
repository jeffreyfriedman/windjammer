package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

type node struct {
	key         int64
	left, right *node
}

func insert(n *node, k int64) (*node, bool) {
	if n == nil {
		return &node{key: k}, true
	}
	var ok bool
	switch {
	case k < n.key:
		n.left, ok = insert(n.left, k)
	case k > n.key:
		n.right, ok = insert(n.right, k)
	}
	return n, ok
}

func contains(n *node, k int64) bool {
	for n != nil {
		switch {
		case k < n.key:
			n = n.left
		case k > n.key:
			n = n.right
		default:
			return true
		}
	}
	return false
}

func minNode(n *node) *node {
	for n.left != nil {
		n = n.left
	}
	return n
}

func remove(n *node, k int64) (*node, bool) {
	if n == nil {
		return nil, false
	}
	var ok bool
	switch {
	case k < n.key:
		n.left, ok = remove(n.left, k)
	case k > n.key:
		n.right, ok = remove(n.right, k)
	default:
		ok = true
		if n.left == nil {
			return n.right, true
		}
		if n.right == nil {
			return n.left, true
		}
		s := minNode(n.right)
		n.key = s.key
		n.right, _ = remove(n.right, s.key)
	}
	return n, ok
}

func inorder(n *node, out *[]string) {
	if n == nil {
		return
	}
	inorder(n.left, out)
	*out = append(*out, strconv.FormatInt(n.key, 10))
	inorder(n.right, out)
}

func height(n *node) int {
	if n == nil {
		return 0
	}
	return 1 + max(height(n.left), height(n.right))
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	var root *node
	for sc.Scan() {
		f := strings.Fields(sc.Text())
		if len(f) == 0 {
			continue
		}
		var x int64
		if len(f) > 1 {
			x, _ = strconv.ParseInt(f[1], 10, 64)
		}
		switch f[0] {
		case "insert":
			var ok bool
			root, ok = insert(root, x)
			fmt.Println(map[bool]string{true: "ok", false: "dup"}[ok])
		case "contains":
			fmt.Println(map[bool]string{true: "yes", false: "no"}[contains(root, x)])
		case "delete":
			var ok bool
			root, ok = remove(root, x)
			fmt.Println(map[bool]string{true: "ok", false: "missing"}[ok])
		case "inorder":
			var out []string
			inorder(root, &out)
			if len(out) == 0 {
				fmt.Println("<empty>")
			} else {
				fmt.Println(strings.Join(out, " "))
			}
		case "height":
			fmt.Println(height(root))
		case "min":
			if root == nil {
				fmt.Println("empty")
			} else {
				fmt.Println(minNode(root).key)
			}
		case "max":
			if root == nil {
				fmt.Println("empty")
			} else {
				n := root
				for n.right != nil {
					n = n.right
				}
				fmt.Println(n.key)
			}
		}
	}
}
