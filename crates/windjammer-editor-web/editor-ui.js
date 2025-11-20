/**
 * Windjammer Browser Editor - UI Controller
 * 
 * Manages the UI interactions and updates:
 * - Hierarchy panel
 * - Inspector panel
 * - Viewport integration
 * - Console output
 */

class EditorUI {
    constructor(editorState, renderer) {
        this.state = editorState;
        this.renderer = renderer;
        this.setupEventListeners();
        this.refresh();
    }

    /**
     * Set up all event listeners
     */
    setupEventListeners() {
        // Top bar buttons
        document.getElementById('new-scene')?.addEventListener('click', () => this.newScene());
        document.getElementById('save-scene')?.addEventListener('click', () => this.saveScene());
        document.getElementById('load-scene')?.addEventListener('click', () => this.loadScene());
        document.getElementById('play-scene')?.addEventListener('click', () => this.playScene());

        // Add entity button
        document.getElementById('add-entity')?.addEventListener('click', () => this.showAddEntityMenu());

        // Keyboard shortcuts
        document.addEventListener('keydown', (e) => this.handleKeyboard(e));
    }

    /**
     * Handle keyboard shortcuts
     */
    handleKeyboard(e) {
        // Ctrl/Cmd + Z = Undo
        if ((e.ctrlKey || e.metaKey) && e.key === 'z' && !e.shiftKey) {
            e.preventDefault();
            this.undo();
        }
        // Ctrl/Cmd + Shift + Z = Redo
        else if ((e.ctrlKey || e.metaKey) && e.key === 'z' && e.shiftKey) {
            e.preventDefault();
            this.redo();
        }
        // Delete = Delete selected entity
        else if (e.key === 'Delete' || e.key === 'Backspace') {
            if (this.state.selectedEntityId) {
                e.preventDefault();
                this.deleteSelectedEntity();
            }
        }
        // Ctrl/Cmd + D = Duplicate entity
        else if ((e.ctrlKey || e.metaKey) && e.key === 'd') {
            e.preventDefault();
            this.duplicateSelectedEntity();
        }
    }

    /**
     * Refresh all UI panels
     */
    refresh() {
        this.refreshHierarchy();
        this.refreshInspector();
        this.refreshViewport();
    }

    /**
     * Refresh the hierarchy panel
     */
    refreshHierarchy() {
        const container = document.getElementById('entity-list');
        if (!container) return;

        container.innerHTML = '';

        this.state.scene.entities.forEach(entity => {
            const item = document.createElement('div');
            item.className = 'entity-item';
            if (entity.id === this.state.selectedEntityId) {
                item.classList.add('selected');
            }

            // Entity icon based on components
            const icon = this.getEntityIcon(entity);
            
            item.innerHTML = `
                <span class="entity-icon">${icon}</span>
                <span class="entity-name">${entity.name}</span>
                <button class="entity-delete" data-id="${entity.id}" title="Delete">×</button>
            `;

            item.addEventListener('click', (e) => {
                if (!e.target.classList.contains('entity-delete')) {
                    this.selectEntity(entity.id);
                }
            });

            const deleteBtn = item.querySelector('.entity-delete');
            deleteBtn.addEventListener('click', (e) => {
                e.stopPropagation();
                this.deleteEntity(entity.id);
            });

            container.appendChild(item);
        });
    }

    /**
     * Get icon for entity based on its components
     */
    getEntityIcon(entity) {
        if (entity.components.some(c => c.type === 'Camera3D')) return '📷';
        if (entity.components.some(c => c.type === 'PointLight')) return '💡';
        if (entity.components.some(c => c.type === 'DirectionalLight')) return '☀️';
        if (entity.components.some(c => c.type === 'Mesh')) return '🎲';
        return '📦';
    }

    /**
     * Refresh the inspector panel
     */
    refreshInspector() {
        const container = document.getElementById('inspector-content');
        if (!container) return;

        const entity = this.state.getSelectedEntity();
        if (!entity) {
            container.innerHTML = '<p style="color: #858585; padding: 12px;">No entity selected</p>';
            return;
        }

        let html = `
            <div class="inspector-header">
                <input type="text" class="entity-name-input" value="${entity.name}" data-id="${entity.id}">
            </div>
            <div class="components-list">
        `;

        entity.components.forEach((component, index) => {
            html += this.renderComponent(entity.id, component, index);
        });

        html += `
            </div>
            <button class="add-component-btn" data-id="${entity.id}">+ Add Component</button>
        `;

        container.innerHTML = html;

        // Set up event listeners for inspector
        this.setupInspectorListeners();
    }

    /**
     * Render a component in the inspector
     */
    renderComponent(entityId, component, index) {
        let html = `
            <div class="component-section">
                <div class="component-header">
                    <span class="component-name">${component.type}</span>
                    <button class="component-remove" data-entity="${entityId}" data-component="${component.type}">×</button>
                </div>
                <div class="component-properties">
        `;

        // Render properties based on component type
        for (const [key, value] of Object.entries(component)) {
            if (key === 'type') continue;

            if (typeof value === 'object' && value !== null) {
                // Nested object (e.g., position, color)
                html += `<div class="property-group">
                    <label>${key}</label>
                    <div class="property-fields">`;
                
                for (const [subKey, subValue] of Object.entries(value)) {
                    html += `
                        <div class="property-field">
                            <label>${subKey.toUpperCase()}</label>
                            <input type="number" step="0.01" value="${subValue}" 
                                   data-entity="${entityId}" 
                                   data-component="${component.type}" 
                                   data-property="${key}.${subKey}">
                        </div>
                    `;
                }
                
                html += `</div></div>`;
            } else {
                // Simple property
                const inputType = typeof value === 'number' ? 'number' : 
                                 typeof value === 'boolean' ? 'checkbox' : 'text';
                const inputValue = typeof value === 'boolean' ? (value ? 'checked' : '') : `value="${value}"`;
                const step = typeof value === 'number' ? 'step="0.01"' : '';
                
                html += `
                    <div class="property-field">
                        <label>${key}</label>
                        <input type="${inputType}" ${step} ${inputValue}
                               data-entity="${entityId}" 
                               data-component="${component.type}" 
                               data-property="${key}">
                    </div>
                `;
            }
        }

        html += `
                </div>
            </div>
        `;

        return html;
    }

    /**
     * Set up inspector event listeners
     */
    setupInspectorListeners() {
        // Entity name input
        const nameInput = document.querySelector('.entity-name-input');
        if (nameInput) {
            nameInput.addEventListener('change', (e) => {
                const entityId = parseInt(e.target.dataset.id);
                this.state.updateEntityName(entityId, e.target.value);
                this.refreshHierarchy();
            });
        }

        // Component property inputs
        const propertyInputs = document.querySelectorAll('.component-properties input');
        propertyInputs.forEach(input => {
            input.addEventListener('change', (e) => {
                const entityId = parseInt(e.target.dataset.entity);
                const componentType = e.target.dataset.component;
                const property = e.target.dataset.property;
                
                let value;
                if (e.target.type === 'checkbox') {
                    value = e.target.checked;
                } else if (e.target.type === 'number') {
                    value = parseFloat(e.target.value);
                } else {
                    value = e.target.value;
                }

                this.state.updateComponentProperty(entityId, componentType, property, value);
                this.refreshViewport();
            });
        });

        // Remove component buttons
        const removeButtons = document.querySelectorAll('.component-remove');
        removeButtons.forEach(btn => {
            btn.addEventListener('click', (e) => {
                const entityId = parseInt(e.target.dataset.entity);
                const componentType = e.target.dataset.component;
                this.state.removeComponent(entityId, componentType);
                this.refresh();
            });
        });

        // Add component button
        const addComponentBtn = document.querySelector('.add-component-btn');
        if (addComponentBtn) {
            addComponentBtn.addEventListener('click', (e) => {
                const entityId = parseInt(e.target.dataset.id);
                this.showAddComponentMenu(entityId);
            });
        }
    }

    /**
     * Refresh the viewport (tell renderer to update)
     */
    refreshViewport() {
        if (this.renderer) {
            this.renderer.updateScene(this.state.scene);
        }
    }

    /**
     * Select an entity
     */
    selectEntity(entityId) {
        this.state.selectEntity(entityId);
        this.refreshHierarchy();
        this.refreshInspector();
    }

    /**
     * Delete an entity
     */
    deleteEntity(entityId) {
        if (confirm('Delete this entity?')) {
            this.state.deleteEntity(entityId);
            this.refresh();
            this.log(`Deleted entity ${entityId}`);
        }
    }

    /**
     * Delete selected entity
     */
    deleteSelectedEntity() {
        if (this.state.selectedEntityId) {
            this.deleteEntity(this.state.selectedEntityId);
        }
    }

    /**
     * Duplicate selected entity
     */
    duplicateSelectedEntity() {
        const entity = this.state.getSelectedEntity();
        if (entity) {
            const newEntity = this.state.createEntity(
                entity.name + " (Copy)",
                JSON.parse(JSON.stringify(entity.components))
            );
            this.selectEntity(newEntity.id);
            this.refresh();
            this.log(`Duplicated entity: ${entity.name}`);
        }
    }

    /**
     * Show add entity menu
     */
    showAddEntityMenu() {
        const entityTypes = [
            { name: 'Empty Entity', components: [] },
            { name: 'Cube', components: ['Mesh', 'Material'] },
            { name: 'Point Light', components: ['PointLight'] },
            { name: 'Directional Light', components: ['DirectionalLight'] },
            { name: 'Camera', components: ['Camera3D'] }
        ];

        const menu = prompt('Entity type:\n' + entityTypes.map((t, i) => `${i + 1}. ${t.name}`).join('\n'));
        const index = parseInt(menu) - 1;

        if (index >= 0 && index < entityTypes.length) {
            const type = entityTypes[index];
            const components = type.components.map(c => this.state.createDefaultComponent(c));
            const entity = this.state.createEntity(type.name, components);
            this.selectEntity(entity.id);
            this.refresh();
            this.log(`Created entity: ${type.name}`);
        }
    }

    /**
     * Show add component menu
     */
    showAddComponentMenu(entityId) {
        const componentTypes = [
            'Mesh',
            'Material',
            'PointLight',
            'DirectionalLight',
            'Camera3D',
            'RigidBody3D',
            'BoxCollider'
        ];

        const menu = prompt('Component type:\n' + componentTypes.map((t, i) => `${i + 1}. ${t}`).join('\n'));
        const index = parseInt(menu) - 1;

        if (index >= 0 && index < componentTypes.length) {
            const componentType = componentTypes[index];
            if (this.state.addComponent(entityId, componentType)) {
                this.refresh();
                this.log(`Added component: ${componentType}`);
            } else {
                this.log(`Failed to add component: ${componentType} (already exists?)`, 'error');
            }
        }
    }

    /**
     * New scene
     */
    newScene() {
        if (confirm('Create new scene? Unsaved changes will be lost.')) {
            this.state.clear();
            this.refresh();
            this.log('Created new scene');
        }
    }

    /**
     * Save scene
     */
    saveScene() {
        const json = this.state.toJSON();
        
        // Save to localStorage
        localStorage.setItem('windjammer_scene', json);
        
        // Also offer download
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `${this.state.scene.name}.json`;
        a.click();
        URL.revokeObjectURL(url);
        
        this.log('Scene saved');
    }

    /**
     * Load scene
     */
    loadScene() {
        const json = prompt('Paste scene JSON:');
        if (json) {
            if (this.state.fromJSON(json)) {
                this.refresh();
                this.log('Scene loaded');
            } else {
                this.log('Failed to load scene', 'error');
            }
        }
    }

    /**
     * Play scene (placeholder)
     */
    playScene() {
        this.log('Play mode not yet implemented', 'warn');
    }

    /**
     * Undo
     */
    undo() {
        if (this.state.undo()) {
            this.refresh();
            this.log('Undo');
        }
    }

    /**
     * Redo
     */
    redo() {
        if (this.state.redo()) {
            this.refresh();
            this.log('Redo');
        }
    }

    /**
     * Log message to console panel
     */
    log(message, type = 'info') {
        const console = document.getElementById('console-output');
        if (!console) return;

        const timestamp = new Date().toLocaleTimeString();
        const colors = {
            info: '#d4d4d4',
            warn: '#ffa500',
            error: '#ff4444'
        };

        const entry = document.createElement('div');
        entry.style.color = colors[type] || colors.info;
        entry.textContent = `[${timestamp}] ${message}`;
        
        console.appendChild(entry);
        console.scrollTop = console.scrollHeight;

        // Limit console entries
        while (console.children.length > 100) {
            console.removeChild(console.firstChild);
        }
    }
}

// Export for use in other modules
window.EditorUI = EditorUI;

