package main

import (
	"bufio"
	"fmt"
	"os"
	"sort"
	"strconv"
	"strings"
)

type task struct {
	id, dur    int
	deps       []int
	start, end int
	started    bool
}

func main() {
	sc := bufio.NewScanner(os.Stdin)
	sc.Scan()
	hdr := strings.Fields(sc.Text())
	workers, _ := strconv.Atoi(hdr[0])
	n, _ := strconv.Atoi(hdr[1])
	tasks := make([]*task, 0, n)
	byID := map[int]*task{}
	for i := 0; i < n && sc.Scan(); i++ {
		f := strings.Fields(sc.Text())
		t := &task{}
		t.id, _ = strconv.Atoi(f[0])
		t.dur, _ = strconv.Atoi(f[1])
		for _, d := range f[2:] {
			v, _ := strconv.Atoi(d)
			t.deps = append(t.deps, v)
		}
		tasks = append(tasks, t)
		byID[t.id] = t
	}
	sort.Slice(tasks, func(i, j int) bool { return tasks[i].id < tasks[j].id })

	now, free, done, makespan := 0, workers, 0, 0
	for done < n {
		for _, t := range tasks {
			if free == 0 {
				break
			}
			if t.started || !ready(t, byID, now) {
				continue
			}
			t.started, t.start, t.end = true, now, now+t.dur
			free--
		}
		// advance to the next finishing time
		next := -1
		for _, t := range tasks {
			if t.started && t.end > now && (next < 0 || t.end < next) {
				next = t.end
			}
		}
		if next < 0 {
			break
		}
		now = next
		for _, t := range tasks {
			if t.started && t.end == now {
				free++
				done++
				makespan = max(makespan, t.end)
			}
		}
	}
	for _, t := range tasks {
		fmt.Println(t.id, t.start, t.end)
	}
	fmt.Println("makespan", makespan)
}

func ready(t *task, byID map[int]*task, now int) bool {
	for _, d := range t.deps {
		dep := byID[d]
		if !dep.started || dep.end > now {
			return false
		}
	}
	return true
}
