// Windjammer Desktop Editor - JavaScript

const { invoke } = window.__TAURI__.core;

// State
let currentFile = null;
let currentProject = null;

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    setupEventListeners();
    loadDefaultProject();
});

// Event Listeners
function setupEventListeners() {
    document.getElementById('new-project').addEventListener('click', newProject);
    document.getElementById('open-project').addEventListener('click', openProject);
    document.getElementById('save-project').addEventListener('click', saveProject);
    document.getElementById('run-project').addEventListener('click', runProject);
}

// New Project
async function newProject() {
    const editor = document.getElementById('code-editor');
    editor.value = `// New Windjammer Project

fn main() {
    println("Hello, Windjammer!")
}
`;
    currentFile = null;
    updateStatus('New project created');
}

// Open Project
async function openProject() {
    try {
        // TODO: Use Tauri dialog API to select file
        updateStatus('Open project dialog - coming soon!');
    } catch (error) {
        console.error('Error opening project:', error);
        updateStatus('Error opening project');
    }
}

// Save Project
async function saveProject() {
    try {
        const editor = document.getElementById('code-editor');
        const content = editor.value;
        
        if (!currentFile) {
            // TODO: Use Tauri dialog API to save file
            updateStatus('Save dialog - coming soon!');
            return;
        }
        
        await invoke('write_file', {
            path: currentFile,
            content: content
        });
        
        updateStatus('Project saved!');
    } catch (error) {
        console.error('Error saving project:', error);
        updateStatus('Error saving project');
    }
}

// Run Project
async function runProject() {
    try {
        const editor = document.getElementById('code-editor');
        const source = editor.value;
        
        updateStatus('Compiling...');
        
        const result = await invoke('compile_windjammer', { source });
        
        updateStatus('✅ Compilation successful!');
        
        // Display success in error panel
        const errorList = document.getElementById('error-list');
        errorList.innerHTML = '<p style="color: green;">No errors! Code compiled successfully.</p>';
        
        console.log('Compiled output:', result);
    } catch (error) {
        console.error('Compilation error:', error);
        updateStatus('❌ Compilation failed');
        
        // Display error in error panel
        const errorList = document.getElementById('error-list');
        errorList.innerHTML = `<div class="error-item">
            <div class="error-message">${error}</div>
        </div>`;
    }
}

// Load Default Project
function loadDefaultProject() {
    const defaultCode = `// Welcome to Windjammer Desktop Editor!
// This is a simple Hello World example.

fn main() {
    println("Hello, Windjammer!")
    
    let name = "World"
    println("Hello, " + name + "!")
    
    // Try creating a game!
    // Uncomment the code below:
    
    // @game
    // struct MyGame {
    //     score: int,
    // }
    // 
    // @init
    // fn init(game: MyGame) {
    //     game.score = 0
    // }
    // 
    // @update
    // fn update(game: MyGame, delta: float) {
    //     game.score += 1
    //     println("Score: " + game.score.to_string())
    // }
}
`;
    
    const editor = document.getElementById('code-editor');
    editor.value = defaultCode;
    
    updateStatus('Loaded default project');
}

// Update Status
function updateStatus(message) {
    const statusText = document.getElementById('status-text');
    statusText.textContent = message;
}

// File Browser (placeholder)
function updateFileBrowser(files) {
    const fileList = document.getElementById('file-list');
    fileList.innerHTML = '';
    
    files.forEach(file => {
        const fileItem = document.createElement('div');
        fileItem.className = 'file-item';
        fileItem.textContent = file.name;
        fileItem.addEventListener('click', () => openFile(file.path));
        fileList.appendChild(fileItem);
    });
}

// Open File
async function openFile(path) {
    try {
        const content = await invoke('read_file', { path });
        const editor = document.getElementById('code-editor');
        editor.value = content;
        currentFile = path;
        updateStatus(`Opened: ${path}`);
    } catch (error) {
        console.error('Error opening file:', error);
        updateStatus('Error opening file');
    }
}

