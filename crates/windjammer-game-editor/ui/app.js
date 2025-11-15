// Windjammer Game Editor - Modern UI
// Proper Tauri API integration with error handling

// Wait for DOM and Tauri to be ready
document.addEventListener('DOMContentLoaded', async () => {
    console.log('DOM loaded, initializing editor...');
    
    // Wait for Tauri API to be available
    let attempts = 0;
    while (!window.__TAURI__ && attempts < 50) {
        await new Promise(resolve => setTimeout(resolve, 100));
        attempts++;
    }
    
    // Check if Tauri API is available
    if (!window.__TAURI__) {
        console.error('Tauri API not available after waiting!');
        alert('ERROR: Tauri API not loaded. Please restart the application.');
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
    
    // Helper functions (defined first)
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
    
    function updateRunningState() {
        runGameBtn.disabled = isRunning;
        stopGameBtn.disabled = !isRunning;
    }
    
    // Event Listeners
    if (newProjectBtn) {
        newProjectBtn.addEventListener('click', createNewProject);
        logToConsole('✓ New Project button listener attached');
    }
    if (openProjectBtn) {
        openProjectBtn.addEventListener('click', openProject);
        logToConsole('✓ Open Project button listener attached');
    }
    if (saveFileBtn) saveFileBtn.addEventListener('click', saveFile);
    if (runGameBtn) runGameBtn.addEventListener('click', runGame);
    if (stopGameBtn) stopGameBtn.addEventListener('click', stopGame);
    if (buildGameBtn) buildGameBtn.addEventListener('click', buildGame);
    
    const clearConsoleBtn = document.getElementById('clear-console');
    if (clearConsoleBtn) clearConsoleBtn.addEventListener('click', clearConsole);
    
    // Modal dialog button listeners
    const closeNewProjectBtn = document.getElementById('close-new-project');
    const cancelNewProjectBtn = document.getElementById('cancel-new-project');
    const confirmNewProjectBtn = document.getElementById('confirm-new-project');
    
    if (closeNewProjectBtn) {
        closeNewProjectBtn.addEventListener('click', closeNewProjectDialog);
        logToConsole('✓ Close new project button listener attached');
    }
    if (cancelNewProjectBtn) {
        cancelNewProjectBtn.addEventListener('click', closeNewProjectDialog);
        logToConsole('✓ Cancel new project button listener attached');
    }
    if (confirmNewProjectBtn) {
        confirmNewProjectBtn.addEventListener('click', confirmNewProject);
        logToConsole('✓ Confirm new project button listener attached');
    }
    
    const closeOpenProjectBtn = document.getElementById('close-open-project');
    const cancelOpenProjectBtn = document.getElementById('cancel-open-project');
    const confirmOpenProjectBtn = document.getElementById('confirm-open-project');
    
    if (closeOpenProjectBtn) {
        closeOpenProjectBtn.addEventListener('click', closeOpenProjectDialog);
        logToConsole('✓ Close open project button listener attached');
    }
    if (cancelOpenProjectBtn) {
        cancelOpenProjectBtn.addEventListener('click', closeOpenProjectDialog);
        logToConsole('✓ Cancel open project button listener attached');
    }
    if (confirmOpenProjectBtn) {
        confirmOpenProjectBtn.addEventListener('click', confirmOpenProject);
        logToConsole('✓ Confirm open project button listener attached');
    }
    
    // Track cursor position in editor
    if (codeEditor) {
        codeEditor.addEventListener('input', updateCursorPosition);
        codeEditor.addEventListener('click', updateCursorPosition);
        codeEditor.addEventListener('keyup', updateCursorPosition);
    }
    
    logToConsole('Editor initialized successfully!');
    updateStatus('Ready');
    
    // Modal Dialog Functions
    function createNewProject() {
        logToConsole('📝 Opening new project dialog...');
        document.getElementById('new-project-dialog').style.display = 'flex';
        setTimeout(() => {
            document.getElementById('project-name-input').focus();
        }, 100);
    }
    
    function closeNewProjectDialog() {
        document.getElementById('new-project-dialog').style.display = 'none';
        logToConsole('✗ New project dialog closed');
    }
    
    async function confirmNewProject() {
        try {
            const projectName = document.getElementById('project-name-input').value.trim();
            const projectPath = document.getElementById('project-path-input').value.trim();
            const template = document.getElementById('template-select').value;
            
            if (!projectName) {
                logToConsole('✗ Project name is required');
                alert('Please enter a project name');
                return;
            }
            
            if (!projectPath) {
                logToConsole('✗ Project path is required');
                alert('Please enter a project path');
                return;
            }
            
            // Close dialog
            document.getElementById('new-project-dialog').style.display = 'none';
            
            updateStatus('Creating project...');
            logToConsole(`Creating ${template} project: ${projectName} at ${projectPath}...`);
            logToConsole(`Invoking Tauri command: create_game_project`);
            
            await invoke('create_game_project', {
                path: projectPath,
                name: projectName,
                template: template
            });
            
            logToConsole(`✓ Project created successfully!`);
            logToConsole(`   Template: ${template}`);
            logToConsole(`   Location: ${projectPath}/${projectName}`);
            updateStatus('Project created');
            
            // Load the project
            currentProject = `${projectPath}/${projectName}`;
            logToConsole(`Loading project files from: ${currentProject}`);
            await loadProjectFiles(currentProject);
            
            // Open the main.wj file
            const mainFile = `${currentProject}/main.wj`;
            logToConsole(`Opening main file: ${mainFile}`);
            await openFile(mainFile);
            
            // Hide welcome screen
            if (welcomeScreen) welcomeScreen.style.display = 'none';
            if (codeEditor) codeEditor.style.display = 'block';
            
            logToConsole(`✓ Project setup complete!`);
            
            // Clear form
            document.getElementById('project-name-input').value = '';
            document.getElementById('project-path-input').value = '/tmp';
            document.getElementById('template-select').value = 'platformer';
            
        } catch (error) {
            logToConsole(`✗ Error creating project: ${error}`);
            logToConsole(`   Error details: ${JSON.stringify(error)}`);
            updateStatus('Error creating project');
            console.error('Create project error:', error);
        }
    }
    
    function openProject() {
        logToConsole('📂 Opening project dialog...');
        document.getElementById('open-project-dialog').style.display = 'flex';
        setTimeout(() => {
            document.getElementById('open-project-path-input').focus();
        }, 100);
    }
    
    function closeOpenProjectDialog() {
        document.getElementById('open-project-dialog').style.display = 'none';
        logToConsole('✗ Open project dialog closed');
    }
    
    async function confirmOpenProject() {
        try {
            const projectPath = document.getElementById('open-project-path-input').value.trim();
            
            if (!projectPath) {
                logToConsole('✗ Project path is required');
                alert('Please enter a project path');
                return;
            }
            
            // Close dialog
            document.getElementById('open-project-dialog').style.display = 'none';
            
            updateStatus('Opening project...');
            logToConsole(`Opening project: ${projectPath}...`);
            logToConsole(`Invoking Tauri command: list_directory`);
            
            currentProject = projectPath;
            await loadProjectFiles(projectPath);
            
            logToConsole(`✓ Project opened successfully!`);
            updateStatus('Project opened');
            
            // Hide welcome screen
            if (welcomeScreen) welcomeScreen.style.display = 'none';
            if (codeEditor) codeEditor.style.display = 'block';
            
            // Clear form
            document.getElementById('open-project-path-input').value = '';
            
        } catch (error) {
            logToConsole(`✗ Error opening project: ${error}`);
            logToConsole(`   Error details: ${JSON.stringify(error)}`);
            updateStatus('Error opening project');
            console.error('Open project error:', error);
        }
    }
    
    async function loadProjectFiles(projectPath) {
        try {
            logToConsole(`Listing directory: ${projectPath}`);
            const files = await invoke('list_directory', { path: projectPath });
            logToConsole(`Found ${files.length} file(s)`);
            
            if (!fileTree) {
                logToConsole('✗ Error: fileTree element not found!');
                return;
            }
            
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
                logToConsole(`  - ${file.is_directory ? '📁' : '📄'} ${file.name}`);
            });
            
        } catch (error) {
            logToConsole(`✗ Error loading files: ${error}`);
            logToConsole(`   Error details: ${JSON.stringify(error)}`);
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
    
    function clearConsole() {
        consoleOutput.textContent = 'Console cleared.\n';
        logToConsole('Ready for new output...');
    }
    
    // Initialize
    logToConsole('Ready to create amazing games!');
    logToConsole('Create a new project or open an existing one to get started.');
});

