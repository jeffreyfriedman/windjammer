/**
 * Windjammer Browser Editor - State Management
 * 
 * Manages the editor's state including:
 * - Scene entities and hierarchy
 * - Selected entity
 * - Component data
 * - Undo/redo history
 */

class EditorState {
    constructor() {
        this.scene = {
            name: "Untitled Scene",
            entities: []
        };
        this.selectedEntityId = null;
        this.nextEntityId = 1;
        this.history = [];
        this.historyIndex = -1;
        this.maxHistory = 50;
        
        // Create default camera entity
        this.createDefaultCamera();
    }

    /**
     * Create a default camera entity
     */
    createDefaultCamera() {
        const camera = {
            id: this.nextEntityId++,
            name: "Main Camera",
            components: [
                {
                    type: "Transform3D",
                    position: { x: 0, y: 2, z: 5 },
                    rotation: { x: 0, y: 0, z: 0 },
                    scale: { x: 1, y: 1, z: 1 }
                },
                {
                    type: "Camera3D",
                    fov: 60,
                    near: 0.1,
                    far: 1000,
                    clearColor: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }
                }
            ]
        };
        this.scene.entities.push(camera);
    }

    /**
     * Create a new entity
     */
    createEntity(name = "New Entity", components = []) {
        const entity = {
            id: this.nextEntityId++,
            name: name,
            components: components.length > 0 ? components : [
                {
                    type: "Transform3D",
                    position: { x: 0, y: 0, z: 0 },
                    rotation: { x: 0, y: 0, z: 0 },
                    scale: { x: 1, y: 1, z: 1 }
                }
            ]
        };
        
        this.scene.entities.push(entity);
        this.pushHistory();
        return entity;
    }

    /**
     * Delete an entity by ID
     */
    deleteEntity(entityId) {
        const index = this.scene.entities.findIndex(e => e.id === entityId);
        if (index !== -1) {
            this.scene.entities.splice(index, 1);
            if (this.selectedEntityId === entityId) {
                this.selectedEntityId = null;
            }
            this.pushHistory();
            return true;
        }
        return false;
    }

    /**
     * Get entity by ID
     */
    getEntity(entityId) {
        return this.scene.entities.find(e => e.id === entityId);
    }

    /**
     * Update entity name
     */
    updateEntityName(entityId, newName) {
        const entity = this.getEntity(entityId);
        if (entity) {
            entity.name = newName;
            this.pushHistory();
            return true;
        }
        return false;
    }

    /**
     * Add component to entity
     */
    addComponent(entityId, componentType) {
        const entity = this.getEntity(entityId);
        if (!entity) return false;

        // Check if component already exists
        if (entity.components.some(c => c.type === componentType)) {
            console.warn(`Entity already has component: ${componentType}`);
            return false;
        }

        // Create default component data based on type
        const component = this.createDefaultComponent(componentType);
        entity.components.push(component);
        this.pushHistory();
        return true;
    }

    /**
     * Remove component from entity
     */
    removeComponent(entityId, componentType) {
        const entity = this.getEntity(entityId);
        if (!entity) return false;

        const index = entity.components.findIndex(c => c.type === componentType);
        if (index !== -1) {
            entity.components.splice(index, 1);
            this.pushHistory();
            return true;
        }
        return false;
    }

    /**
     * Update component property
     */
    updateComponentProperty(entityId, componentType, property, value) {
        const entity = this.getEntity(entityId);
        if (!entity) return false;

        const component = entity.components.find(c => c.type === componentType);
        if (!component) return false;

        // Handle nested properties (e.g., "position.x")
        const parts = property.split('.');
        let target = component;
        
        for (let i = 0; i < parts.length - 1; i++) {
            if (!target[parts[i]]) {
                target[parts[i]] = {};
            }
            target = target[parts[i]];
        }
        
        target[parts[parts.length - 1]] = value;
        this.pushHistory();
        return true;
    }

    /**
     * Create default component data
     */
    createDefaultComponent(componentType) {
        const defaults = {
            "Transform3D": {
                type: "Transform3D",
                position: { x: 0, y: 0, z: 0 },
                rotation: { x: 0, y: 0, z: 0 },
                scale: { x: 1, y: 1, z: 1 }
            },
            "Mesh": {
                type: "Mesh",
                meshType: "cube",
                castShadows: true,
                receiveShadows: true
            },
            "Material": {
                type: "Material",
                albedo: { r: 1.0, g: 1.0, b: 1.0, a: 1.0 },
                metallic: 0.0,
                roughness: 0.5,
                emissive: { r: 0.0, g: 0.0, b: 0.0 }
            },
            "PointLight": {
                type: "PointLight",
                color: { r: 1.0, g: 1.0, b: 1.0 },
                intensity: 1.0,
                range: 10.0
            },
            "DirectionalLight": {
                type: "DirectionalLight",
                color: { r: 1.0, g: 1.0, b: 1.0 },
                intensity: 1.0,
                direction: { x: -0.3, y: -1.0, z: -0.3 }
            },
            "Camera3D": {
                type: "Camera3D",
                fov: 60,
                near: 0.1,
                far: 1000,
                clearColor: { r: 0.1, g: 0.1, b: 0.1, a: 1.0 }
            },
            "RigidBody3D": {
                type: "RigidBody3D",
                mass: 1.0,
                friction: 0.5,
                restitution: 0.3,
                isKinematic: false
            },
            "BoxCollider": {
                type: "BoxCollider",
                size: { x: 1.0, y: 1.0, z: 1.0 },
                offset: { x: 0.0, y: 0.0, z: 0.0 }
            }
        };

        return defaults[componentType] || { type: componentType };
    }

    /**
     * Select an entity
     */
    selectEntity(entityId) {
        this.selectedEntityId = entityId;
    }

    /**
     * Get selected entity
     */
    getSelectedEntity() {
        return this.selectedEntityId ? this.getEntity(this.selectedEntityId) : null;
    }

    /**
     * Push current state to history
     */
    pushHistory() {
        // Remove any history after current index
        this.history = this.history.slice(0, this.historyIndex + 1);
        
        // Add current state
        this.history.push(JSON.parse(JSON.stringify(this.scene)));
        
        // Limit history size
        if (this.history.length > this.maxHistory) {
            this.history.shift();
        } else {
            this.historyIndex++;
        }
    }

    /**
     * Undo last action
     */
    undo() {
        if (this.historyIndex > 0) {
            this.historyIndex--;
            this.scene = JSON.parse(JSON.stringify(this.history[this.historyIndex]));
            return true;
        }
        return false;
    }

    /**
     * Redo last undone action
     */
    redo() {
        if (this.historyIndex < this.history.length - 1) {
            this.historyIndex++;
            this.scene = JSON.parse(JSON.stringify(this.history[this.historyIndex]));
            return true;
        }
        return false;
    }

    /**
     * Serialize scene to JSON
     */
    toJSON() {
        return JSON.stringify(this.scene, null, 2);
    }

    /**
     * Load scene from JSON
     */
    fromJSON(json) {
        try {
            const data = JSON.parse(json);
            this.scene = data;
            this.selectedEntityId = null;
            this.history = [];
            this.historyIndex = -1;
            this.pushHistory();
            return true;
        } catch (e) {
            console.error("Failed to load scene from JSON:", e);
            return false;
        }
    }

    /**
     * Clear scene
     */
    clear() {
        this.scene = {
            name: "Untitled Scene",
            entities: []
        };
        this.selectedEntityId = null;
        this.nextEntityId = 1;
        this.history = [];
        this.historyIndex = -1;
        this.createDefaultCamera();
        this.pushHistory();
    }
}

// Export for use in other modules
window.EditorState = EditorState;

