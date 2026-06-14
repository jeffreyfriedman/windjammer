// Windjammer std::subprocess backend for Go (os/exec).
package wjsubprocess

import (
	"bufio"
	"fmt"
	"os/exec"
)

type Handle struct {
	ID int
}

var sessions = map[int]*exec.Cmd{}
var readers = map[int]*bufio.Reader{}
var writers = map[int]exec.StdinPipeWriter{}
var nextID = 1

func Spawn(program string, args []string) (Handle, error) {
	cmd := exec.Command(program, args...)
	stdin, err := cmd.StdinPipe()
	if err != nil {
		return Handle{}, err
	}
	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return Handle{}, err
	}
	if err := cmd.Start(); err != nil {
		return Handle{}, err
	}
	id := nextID
	nextID++
	sessions[id] = cmd
	readers[id] = bufio.NewReader(stdout)
	// store stdin writer when Go codegen wires full MCP support
	_ = stdin
	return Handle{ID: id}, nil
}

func WriteLine(h Handle, line string) error {
	return fmt.Errorf("wjsubprocess.WriteLine: wire via Go codegen import")
}
