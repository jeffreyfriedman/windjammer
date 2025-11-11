// Windjammer Game Editor - Modern UI
// Proper Tauri API integration with error handling

// Wait for DOM and Tauri to be ready
document.addEventListener('DOMContentLoaded', async () => {
    console.log('DOM loaded, initializing editor...');
    
    // Check if Tauri API is available
    if (!window.__TAURI__) {
        console.error('Tauri API not available!');
        logToConsole('ERROR: Tauri API not loaded. Running in browser mode (limited functionality).');
        return;
    }
    
    const { invoke } = window.__TAURI__.core;
    console.log('Tauri API loaded successfully');
    
    // State
    let currentFile = '';
    let currentProject = '';
    let isRunning = false;
    let openFiles = [];
    
    // DOM Elements
    const newProjectBtn = document.getElementById('new-project');
    const openProjectBtn = document.getElementById('open-project');
    const saveFileBtn = document.getElementById('save-file');
    const runGameBtn = document.getElementById('run-game');
    const stopGameBtn = document.getElementById('stop-game');
    const buildGameBtn = document.getElementById('build-game');
    const codeEditor = document.getElementById('code-editor');
    const consoleOutput = document.getElementById('console-output');
    const fileTree = document.getElementById('file-tree');
    const welcomeScreen = document.getElementById('welcome-screen');
    const statusText = document.getElementById('status-text');
    const fileInfo = document.getElementById('file-info');
    
    // Event Listeners
    newProjectBtn.addEventListener('click', createNewProject);
    openProjectBtn.addEventListener('click', openProject);
    saveFileBtn.addEventListener('click', saveFile);
    runGameBtn.addEventListener('click', runGame);
    stopGameBtn.addEventListener('click', stopGame);
    buildGameBtn.addEventListener('click', buildGame);
    
    // Track cursor position in editor
    codeEditor.addEventListener('input', updateCursorPosition);
    codeEditor.addEventListener('click', updateCursorPosition);
    codeEditor.addEventListener('keyup', updateCursorPosition);
    
    logToConsole('Editor initialized successfully!');
    updateStatus('Ready');
    
    // Functions
    async function createNewProject() {
        try {
            const projectName = prompt('Enter project name:');
            if (!projectName) return;
            
            const projectPath = prompt('Enter project path:', '/tmp');
            if (!projectPath) return;
            
            updateStatus('Creating project...');
            logToConsole(`Creating project: ${projectName} at ${projectPath}...`);
            
            await invoke('create_game_project', {
                path: projectPath,
                name: projectName
            });
            
            logToConsole(`✓ Project created successfully!`);
            updateStatus('Project created');
            
            // Load the project
            currentProject = `${projectPath}/${projectName}`;
            await loadProjectFiles(currentProject);
            
            // Hide welcome screen
            welcomeScreen.style.display = 'none';
            codeEditor.style.display = 'block';
            
        } catch (error) {
            logToConsole(`✗ Error: ${error}`);
            updateStatus('Error creating project');
            console.error('Create project error:', error);
        }
    }
    
    async function openProject() {
        try {
            const projectPath = prompt('Enter project path:');
            if (!projectPath) return;
            
            updateStatus('Opening project...');
            logToConsole(`Opening project: ${projectPath}...`);
            
            currentProject = projectPath;
            await loadProjectFiles(projectPath);
            
            logToConsole(`✓ Project opened successfully!`);
            updateStatus('Project opened');
            
            // Hide welcome screen
            welcomeScreen.style.display = 'none';
            codeEditor.style.display = 'block';
            
        } catch (error) {
            logToConsole(`✗ Error: ${error}`);
            updateStatus('Error opening project');
            console.error('Open project error:', error);
        }
    }
    
    async function loadProjectFiles(projectPath) {
        try {
            const files = await invoke('list_directory', { path: projectPath });
            
            fileTree.innerHTML = '';
            files.forEach(file => {
                const fileItem = document.createElement('div');
                fileItem.className = 'tree-item';
                
                const icon = document.createElement('span');
                icon.className = 'tree-icon';
                icon.textContent = file.is_directory ? '📁' : '📄';
                
                const name = document.createElement('span');
                name.textContent = file.name;
                
                fileItem.appendChild(icon);
                fileItem.appendChild(name);
                fileItem.onclick = () => openFile(file.path);
                
                fileTree.appendChild(fileItem);
            });
            
        } catch (error) {
            logToConsole(`✗ Error loading files: ${error}`);
            console.error('Load files error:', error);
        }
    }
    
    async function openFile(filePath) {
        try {
            updateStatus('Opening file...');
            logToConsole(`Opening file: ${filePath}...`);
            
            const content = await invoke('read_file', { path: filePath });
            
            currentFile = filePath;
            codeEditor.value = content;
            
            // Update file info
            const fileName = filePath.split('/').pop();
            fileInfo.textContent = fileName;
            
            // Highlight active file
            document.querySelectorAll('.tree-item').forEach(item => {
                item.classList.remove('active');
                if (item.textContent.includes(fileName)) {
                    item.classList.add('active');
                }
            });
            
            logToConsole(`✓ File opened successfully!`);
            updateStatus('File opened');
            
        } catch (error) {
            logToConsole(`✗ Error opening file: ${error}`);
            updateStatus('Error opening file');
            console.error('Open file error:', error);
        }
    }
    
    async function saveFile() {
        if (!currentFile) {
            logToConsole('✗ No file open');
            updateStatus('No file to save');
            return;
        }
        
        try {
            updateStatus('Saving file...');
            logToConsole(`Saving file: ${currentFile}...`);
            
            await invoke('write_file', {
                path: currentFile,
                content: codeEditor.value
            });
            
            logToConsole(`✓ File saved successfully!`);
            updateStatus('File saved');
            
        } catch (error) {
            logToConsole(`✗ Error saving file: ${error}`);
            updateStatus('Error saving file');
            console.error('Save file error:', error);
        }
    }
    
    async function runGame() {
        if (!currentProject) {
            logToConsole('✗ No project open');
            updateStatus('No project to run');
            return;
        }
        
        try {
            updateStatus('Running game...');
            logToConsole('Compiling and running game...');
            logToConsole('─'.repeat(50));
            
            isRunning = true;
            updateRunningState();
            
            const result = await invoke('run_game', { projectPath: currentProject });
            logToConsole(result);
            
            updateStatus('Game running');
            
        } catch (error) {
            logToConsole(`✗ Error: ${error}`);
            updateStatus('Error running game');
            isRunning = false;
            updateRunningState();
            console.error('Run game error:', error);
        }
    }
    
    async function stopGame() {
        try {
            updateStatus('Stopping game...');
            logToConsole('Stopping game...');
            
            await invoke('stop_game');
            
            isRunning = false;
            updateRunningState();
            
            logToConsole('✓ Game stopped');
            updateStatus('Game stopped');
            
        } catch (error) {
            logToConsole(`✗ Error: ${error}`);
            updateStatus('Error stopping game');
            console.error('Stop game error:', error);
        }
    }
    
    async function buildGame() {
        if (!currentProject) {
            logToConsole('✗ No project open');
            updateStatus('No project to build');
            return;
        }
        
        try {
            updateStatus('Building game...');
            logToConsole('Building game...');
            logToConsole('─'.repeat(50));
            
            const result = await invoke('run_game', { projectPath: currentProject });
            logToConsole(result);
            
            updateStatus('Build complete');
            
        } catch (error) {
            logToConsole(`✗ Build error: ${error}`);
            updateStatus('Build failed');
            console.error('Build error:', error);
        }
    }
    
    function updateRunningState() {
        runGameBtn.disabled = isRunning;
        stopGameBtn.disabled = !isRunning;
    }
    
    function logToConsole(message) {
        const timestamp = new Date().toLocaleTimeString();
        consoleOutput.textContent += `[${timestamp}] ${message}\n`;
        consoleOutput.scrollTop = consoleOutput.scrollHeight;
    }
    
    function updateStatus(message) {
        statusText.textContent = message;
    }
    
    function updateCursorPosition() {
        const text = codeEditor.value;
        const cursorPos = codeEditor.selectionStart;
        
        const lines = text.substr(0, cursorPos).split('\n');
        const line = lines.length;
        const col = lines[lines.length - 1].length + 1;
        
        document.getElementById('cursor-position').textContent = `Ln ${line}, Col ${col}`;
    }
    
    // Initialize
    logToConsole('Ready to create amazing games!');
    logToConsole('Create a new project or open an existing one to get started.');
});

