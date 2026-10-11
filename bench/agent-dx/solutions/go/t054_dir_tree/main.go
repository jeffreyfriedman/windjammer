package main

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

var dirs, files int

func walk(path string, depth int) {
	entries, _ := os.ReadDir(path) // ReadDir returns entries sorted by name
	for _, e := range entries {
		indent := strings.Repeat("  ", depth)
		if e.IsDir() {
			dirs++
			fmt.Printf("%s%s/\n", indent, e.Name())
			walk(filepath.Join(path, e.Name()), depth+1)
			continue
		}
		files++
		info, err := e.Info()
		var size int64
		if err == nil {
			size = info.Size()
		}
		fmt.Printf("%s%s (%d bytes)\n", indent, e.Name(), size)
	}
}

func main() {
	walk(".", 0)
	fmt.Printf("%d directories, %d files\n", dirs, files)
}
