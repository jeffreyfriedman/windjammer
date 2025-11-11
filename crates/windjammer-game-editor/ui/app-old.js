// Windjammer Game Editor - Frontend Logic

// Check if Tauri API is available
if (!window.__TAURI__) {
    console.error('Tauri API not available!');
    alert('Error: Tauri API not loaded. Please check the application configuration.');
}

const { invoke } = window.__TAURI__?.core || {};

// State
let currentFile = '';
let currentProject = '';
let isRunning = false;

// DOM Elements
const newProjectBtn = document.getElementById('new-project');
const openProjectBtn = document.getElementById('open-project');
const saveFileBtn = document.getElementById('save-file');
const runGameBtn = document.getElementById('run-game');
const stopGameBtn = document.getElementById('stop-game');
const codeEditor = document.getElementById('code-editor');
const consoleOutput = document.getElementById('console-output');
const editorTitle = document.getElementById('editor-title');
const fileList = document.getElementById('file-list');
const previewContent = document.getElementById('preview-content');

// Event Listeners
newProjectBtn.addEventListener('click', createNewProject);
openProjectBtn.addEventListener('click', openProject);
saveFileBtn.addEventListener('click', saveFile);
runGameBtn.addEventListener('click', runGame);
stopGameBtn.addEventListener('click', stopGame);

// Functions
async function createNewProject() {
    try {
        const projectName = prompt('Enter project name:');
        if (!projectName) return;
        
        const projectPath = prompt('Enter project path:', '/tmp');
        if (!projectPath) return;
        
        logToConsole(`Creating project: ${projectName} at ${projectPath}...`);
        
        await invoke('create_game_project', {
            path: projectPath,
            name: projectName
        });
        
        logToConsole(`✓ Project created successfully!`);
        
        // Load the project
        currentProject = `${projectPath}/${projectName}`;
        await loadProjectFiles(currentProject);
        
    } catch (error) {
        logToConsole(`✗ Error: ${error}`);
    }
}

async function openProject() {
    try {
        const projectPath = prompt('Enter project path:');
        if (!projectPath) return;
        
        logToConsole(`Opening project: ${projectPath}...`);
        currentProject = projectPath;
        await loadProjectFiles(projectPath);
        logToConsole(`✓ Project opened successfully!`);
        
    } catch (error) {
        logToConsole(`✗ Error: ${error}`);
    }
}

async function loadProjectFiles(projectPath) {
    try {
        const files = await invoke('list_directory', { path: projectPath });
        
        fileList.innerHTML = '';
        files.forEach(file => {
            const fileItem = document.createElement('div');
            fileItem.className = 'file-item';
            fileItem.textContent = file.name;
            fileItem.onclick = () => openFile(file.path);
            fileList.appendChild(fileItem);
        });
        
    } catch (error) {
        logToConsole(`✗ Error loading files: ${error}`);
    }
}

async function openFile(filePath) {
    try {
        logToConsole(`Opening file: ${filePath}...`);
        const content = await invoke('read_file', { path: filePath });
        
        currentFile = filePath;
        codeEditor.value = content;
        editorTitle.textContent = filePath.split('/').pop();
        
        // Highlight active file
        document.querySelectorAll('.file-item').forEach(item => {
            item.classList.remove('active');
            if (item.textContent === filePath.split('/').pop()) {
                item.classList.add('active');
            }
        });
        
        logToConsole(`✓ File opened successfully!`);
        
    } catch (error) {
        logToConsole(`✗ Error opening file: ${error}`);
    }
}

async function saveFile() {
    if (!currentFile) {
        logToConsole('✗ No file open');
        return;
    }
    
    try {
        logToConsole(`Saving file: ${currentFile}...`);
        await invoke('write_file', {
            path: currentFile,
            content: codeEditor.value
        });
        logToConsole(`✓ File saved successfully!`);
        
    } catch (error) {
        logToConsole(`✗ Error saving file: ${error}`);
    }
}

async function runGame() {
    if (!currentProject) {
        logToConsole('✗ No project open');
        return;
    }
    
    try {
        logToConsole('Compiling and running game...');
        logToConsole('─'.repeat(50));
        
        isRunning = true;
        updateRunningState();
        
        const result = await invoke('run_game', { projectPath: currentProject });
        logToConsole(result);
        
        previewContent.innerHTML = '<p style="color: #4ec9b0;">✓ Game is running!</p>';
        
    } catch (error) {
        logToConsole(`✗ Error: ${error}`);
        isRunning = false;
        updateRunningState();
    }
}

async function stopGame() {
    try {
        logToConsole('Stopping game...');
        await invoke('stop_game');
        
        isRunning = false;
        updateRunningState();
        
        previewContent.innerHTML = '<p class="placeholder">Click \'Run\' to start your game</p>';
        logToConsole('✓ Game stopped');
        
    } catch (error) {
        logToConsole(`✗ Error: ${error}`);
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

// Initialize
logToConsole('Editor ready!');
logToConsole('Create a new project or open an existing one to get started.');

