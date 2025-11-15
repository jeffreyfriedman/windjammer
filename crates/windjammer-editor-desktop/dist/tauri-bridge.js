// Tauri Bridge for Windjammer Editor
// This bridges WASM calls to Tauri backend commands

// Check if running in Tauri
const isTauri = window.__TAURI__ !== undefined;

if (!isTauri) {
    console.log('⚠️  Not running in Tauri, using localStorage fallback');
} else {
    console.log('✅ Running in Tauri, using native file system');
}

const { invoke } = isTauri ? window.__TAURI__.core : { invoke: () => Promise.reject('Not in Tauri') };

// Store for synchronous results (hack until we have async/await in Windjammer)
window.__TAURI_RESULTS__ = {};
let resultId = 0;

// Override the WASM fs/process implementations with Tauri calls
window.__WINDJAMMER_TAURI__ = {
    isTauri,
    
    // ============================================================================
    // SYNCHRONOUS WRAPPERS (for current Windjammer sync API)
    // ============================================================================
    
    writeFileSync(path, content) {
        console.log(`📝 Tauri writeFile: ${path}`);
        invoke('write_file', { path, content })
            .then(() => console.log(`✅ File written: ${path}`))
            .catch(err => console.error(`❌ Write failed: ${err}`));
        return { ok: true }; // Return immediately (async operation queued)
    },
    
    createDirectorySync(path) {
        console.log(`📁 Tauri createDirectory: ${path}`);
        invoke('create_directory', { path })
            .then(() => console.log(`✅ Directory created: ${path}`))
            .catch(err => console.error(`❌ Create dir failed: ${err}`));
        return { ok: true }; // Return immediately
    },
    
    executeCommandSync(command, args) {
        console.log(`🚀 Tauri execute: ${command} ${args.join(' ')}`);
        invoke('execute_command', { command, args })
            .then(output => {
                console.log(`✅ Command output:`, output);
                // Store result for potential retrieval
                window.__TAURI_RESULTS__[`${command}_${Date.now()}`] = output;
            })
            .catch(err => console.error(`❌ Execute failed: ${err}`));
        return { 
            ok: true, 
            value: { 
                stdout: `Executing: ${command} ${args.join(' ')}...`, 
                stderr: '', 
                exit_code: 0 
            }
        };
    },
    
    // ============================================================================
    // ASYNC VERSIONS (for future use when Windjammer has async/await)
    // ============================================================================
    
    async readFile(path) {
        try {
            const content = await invoke('read_file', { path });
            return { ok: true, value: content };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async writeFile(path, content) {
        try {
            await invoke('write_file', { path, content });
            return { ok: true };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async createDirectory(path) {
        try {
            await invoke('create_directory', { path });
            return { ok: true };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async listDirectory(path) {
        try {
            const entries = await invoke('list_directory', { path });
            return { ok: true, value: entries };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async deleteFile(path) {
        try {
            await invoke('delete_file', { path });
            return { ok: true };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async fileExists(path) {
        try {
            const exists = await invoke('file_exists', { path });
            return { ok: true, value: exists };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    // ============================================================================
    // PROCESS OPERATIONS
    // ============================================================================
    
    async executeCommand(command, args) {
        try {
            const output = await invoke('execute_command', { command, args });
            return { 
                ok: true, 
                value: {
                    stdout: output.stdout,
                    stderr: output.stderr,
                    exit_code: output.exit_code
                }
            };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async currentDir() {
        try {
            const dir = await invoke('current_dir');
            return { ok: true, value: dir };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    async setCurrentDir(path) {
        try {
            await invoke('set_current_dir', { path });
            return { ok: true };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    },
    
    // ============================================================================
    // DIALOG OPERATIONS
    // ============================================================================
    
    async showMessage(message) {
        try {
            await invoke('show_message', { message });
            return { ok: true };
        } catch (error) {
            return { ok: false, error: error.toString() };
        }
    }
};

console.log('✅ Tauri bridge initialized');

