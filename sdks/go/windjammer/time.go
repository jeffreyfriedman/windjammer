package windjammer

import "fmt"

// Time represents time information for the current frame
type Time struct {
	DeltaSeconds float32 // Time since last frame in seconds
	TotalSeconds float32 // Total time since application start in seconds
	FrameCount   int     // Current frame number
}

// String returns a string representation of Time
func (t *Time) String() string {
	return fmt.Sprintf("Time(delta=%.3f, total=%.3f)", t.DeltaSeconds, t.TotalSeconds)
}

