package windjammer

import "fmt"

// SystemFunc is a function that runs every frame
type SystemFunc func()

// SystemFuncWithTime is a function that runs every frame with time parameter
type SystemFuncWithTime func(*Time)

// App is the main application struct for Windjammer games
type App struct {
	systems         []SystemFunc
	systemsWithTime []SystemFuncWithTime
	startupSystems  []SystemFunc
	shutdownSystems []SystemFunc
	running         bool
}

// NewApp creates a new Windjammer application
func NewApp() *App {
	fmt.Println("[Windjammer] Initializing application...")
	return &App{
		systems:         make([]SystemFunc, 0),
		systemsWithTime: make([]SystemFuncWithTime, 0),
		startupSystems:  make([]SystemFunc, 0),
		shutdownSystems: make([]SystemFunc, 0),
		running:         false,
	}
}

// AddSystem adds a system that runs every frame
func (a *App) AddSystem(system SystemFunc) *App {
	a.systems = append(a.systems, system)
	return a
}

// AddSystemWithTime adds a system with time parameter that runs every frame
func (a *App) AddSystemWithTime(system SystemFuncWithTime) *App {
	a.systemsWithTime = append(a.systemsWithTime, system)
	return a
}

// AddStartupSystem adds a startup system that runs once at the beginning
func (a *App) AddStartupSystem(system SystemFunc) *App {
	a.startupSystems = append(a.startupSystems, system)
	return a
}

// AddShutdownSystem adds a shutdown system that runs once at the end
func (a *App) AddShutdownSystem(system SystemFunc) *App {
	a.shutdownSystems = append(a.shutdownSystems, system)
	return a
}

// Run runs the application
func (a *App) Run() {
	fmt.Printf("[Windjammer] Starting application with %d systems\n", len(a.systems)+len(a.systemsWithTime))

	// Run startup systems
	for _, system := range a.startupSystems {
		system()
	}

	a.running = true

	// TODO: Start actual game loop with CGO
	fmt.Println("[Windjammer] Running systems...")
	time := &Time{DeltaSeconds: 0.016, TotalSeconds: 0.0, FrameCount: 0}

	for _, system := range a.systems {
		system()
	}

	for _, system := range a.systemsWithTime {
		system(time)
	}

	// Run shutdown systems
	for _, system := range a.shutdownSystems {
		system()
	}

	fmt.Println("[Windjammer] Application finished")
	a.running = false
}

// IsRunning checks if the application is currently running
func (a *App) IsRunning() bool {
	return a.running
}

// Quit requests the application to quit
func (a *App) Quit() {
	a.running = false
	fmt.Println("[Windjammer] Quit requested")
}

