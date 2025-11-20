/**
 * WebGL Renderer for Windjammer Editor
 * 
 * Provides 3D rendering capabilities for the scene viewport.
 */

class WebGLRenderer {
    constructor(canvas) {
        this.canvas = canvas;
        this.gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
        
        if (!this.gl) {
            console.error('WebGL not supported');
            return;
        }
        
        this.entities = [];
        this.camera = {
            position: [0, 5, 10],
            target: [0, 0, 0],
            fov: 60,
            near: 0.1,
            far: 1000
        };
        
        this.initShaders();
        this.initBuffers();
        this.initMatrices();
        
        console.log('[WebGL] Renderer initialized');
    }
    
    initShaders() {
        const gl = this.gl;
        
        // Vertex shader
        const vsSource = `
            attribute vec3 aPosition;
            attribute vec3 aNormal;
            attribute vec2 aTexCoord;
            
            uniform mat4 uModelMatrix;
            uniform mat4 uViewMatrix;
            uniform mat4 uProjectionMatrix;
            uniform mat4 uNormalMatrix;
            
            varying vec3 vNormal;
            varying vec3 vPosition;
            varying vec2 vTexCoord;
            
            void main() {
                vec4 worldPosition = uModelMatrix * vec4(aPosition, 1.0);
                vPosition = worldPosition.xyz;
                vNormal = (uNormalMatrix * vec4(aNormal, 0.0)).xyz;
                vTexCoord = aTexCoord;
                gl_Position = uProjectionMatrix * uViewMatrix * worldPosition;
            }
        `;
        
        // Fragment shader with basic PBR
        const fsSource = `
            precision mediump float;
            
            varying vec3 vNormal;
            varying vec3 vPosition;
            varying vec2 vTexCoord;
            
            uniform vec3 uCameraPosition;
            uniform vec3 uLightPosition;
            uniform vec3 uLightColor;
            uniform vec3 uAlbedo;
            uniform float uMetallic;
            uniform float uRoughness;
            
            const float PI = 3.14159265359;
            
            void main() {
                vec3 N = normalize(vNormal);
                vec3 V = normalize(uCameraPosition - vPosition);
                vec3 L = normalize(uLightPosition - vPosition);
                vec3 H = normalize(V + L);
                
                // Simple PBR approximation
                float NdotL = max(dot(N, L), 0.0);
                float NdotH = max(dot(N, H), 0.0);
                float NdotV = max(dot(N, V), 0.0);
                
                // Diffuse
                vec3 diffuse = uAlbedo * NdotL;
                
                // Specular (simplified)
                float spec = pow(NdotH, (1.0 - uRoughness) * 128.0);
                vec3 specular = uLightColor * spec * uMetallic;
                
                // Ambient
                vec3 ambient = uAlbedo * 0.1;
                
                vec3 color = ambient + (diffuse + specular) * uLightColor;
                
                gl_FragColor = vec4(color, 1.0);
            }
        `;
        
        // Compile shaders
        const vertexShader = this.compileShader(vsSource, gl.VERTEX_SHADER);
        const fragmentShader = this.compileShader(fsSource, gl.FRAGMENT_SHADER);
        
        // Create program
        this.program = gl.createProgram();
        gl.attachShader(this.program, vertexShader);
        gl.attachShader(this.program, fragmentShader);
        gl.linkProgram(this.program);
        
        if (!gl.getProgramParameter(this.program, gl.LINK_STATUS)) {
            console.error('Shader program failed to link:', gl.getProgramInfoLog(this.program));
            return;
        }
        
        // Get attribute and uniform locations
        this.locations = {
            attributes: {
                position: gl.getAttribLocation(this.program, 'aPosition'),
                normal: gl.getAttribLocation(this.program, 'aNormal'),
                texCoord: gl.getAttribLocation(this.program, 'aTexCoord')
            },
            uniforms: {
                modelMatrix: gl.getUniformLocation(this.program, 'uModelMatrix'),
                viewMatrix: gl.getUniformLocation(this.program, 'uViewMatrix'),
                projectionMatrix: gl.getUniformLocation(this.program, 'uProjectionMatrix'),
                normalMatrix: gl.getUniformLocation(this.program, 'uNormalMatrix'),
                cameraPosition: gl.getUniformLocation(this.program, 'uCameraPosition'),
                lightPosition: gl.getUniformLocation(this.program, 'uLightPosition'),
                lightColor: gl.getUniformLocation(this.program, 'uLightColor'),
                albedo: gl.getUniformLocation(this.program, 'uAlbedo'),
                metallic: gl.getUniformLocation(this.program, 'uMetallic'),
                roughness: gl.getUniformLocation(this.program, 'uRoughness')
            }
        };
    }
    
    compileShader(source, type) {
        const gl = this.gl;
        const shader = gl.createShader(type);
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
            console.error('Shader compilation failed:', gl.getShaderInfoLog(shader));
            gl.deleteShader(shader);
            return null;
        }
        
        return shader;
    }
    
    initBuffers() {
        // Create cube mesh
        this.cubeBuffer = this.createCube();
        this.sphereBuffer = this.createSphere(0.5, 16, 16);
        this.planeBuffer = this.createPlane(10);
    }
    
    createCube() {
        const gl = this.gl;
        
        // Cube vertices (position + normal)
        const vertices = new Float32Array([
            // Front face
            -0.5, -0.5,  0.5,  0, 0, 1,
             0.5, -0.5,  0.5,  0, 0, 1,
             0.5,  0.5,  0.5,  0, 0, 1,
            -0.5,  0.5,  0.5,  0, 0, 1,
            // Back face
            -0.5, -0.5, -0.5,  0, 0, -1,
            -0.5,  0.5, -0.5,  0, 0, -1,
             0.5,  0.5, -0.5,  0, 0, -1,
             0.5, -0.5, -0.5,  0, 0, -1,
            // Top face
            -0.5,  0.5, -0.5,  0, 1, 0,
            -0.5,  0.5,  0.5,  0, 1, 0,
             0.5,  0.5,  0.5,  0, 1, 0,
             0.5,  0.5, -0.5,  0, 1, 0,
            // Bottom face
            -0.5, -0.5, -0.5,  0, -1, 0,
             0.5, -0.5, -0.5,  0, -1, 0,
             0.5, -0.5,  0.5,  0, -1, 0,
            -0.5, -0.5,  0.5,  0, -1, 0,
            // Right face
             0.5, -0.5, -0.5,  1, 0, 0,
             0.5,  0.5, -0.5,  1, 0, 0,
             0.5,  0.5,  0.5,  1, 0, 0,
             0.5, -0.5,  0.5,  1, 0, 0,
            // Left face
            -0.5, -0.5, -0.5, -1, 0, 0,
            -0.5, -0.5,  0.5, -1, 0, 0,
            -0.5,  0.5,  0.5, -1, 0, 0,
            -0.5,  0.5, -0.5, -1, 0, 0
        ]);
        
        const indices = new Uint16Array([
            0, 1, 2,  0, 2, 3,    // Front
            4, 5, 6,  4, 6, 7,    // Back
            8, 9, 10, 8, 10, 11,  // Top
            12, 13, 14, 12, 14, 15, // Bottom
            16, 17, 18, 16, 18, 19, // Right
            20, 21, 22, 20, 22, 23  // Left
        ]);
        
        const vertexBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
        
        const indexBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
        
        return {
            vertexBuffer,
            indexBuffer,
            vertexCount: indices.length
        };
    }
    
    createSphere(radius, latBands, longBands) {
        // Simplified sphere generation
        // TODO: Implement proper sphere mesh
        return this.cubeBuffer; // Placeholder
    }
    
    createPlane(size) {
        const gl = this.gl;
        const half = size / 2;
        
        const vertices = new Float32Array([
            -half, 0, -half,  0, 1, 0,
             half, 0, -half,  0, 1, 0,
             half, 0,  half,  0, 1, 0,
            -half, 0,  half,  0, 1, 0
        ]);
        
        const indices = new Uint16Array([0, 1, 2, 0, 2, 3]);
        
        const vertexBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, vertexBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
        
        const indexBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
        
        return {
            vertexBuffer,
            indexBuffer,
            vertexCount: indices.length
        };
    }
    
    initMatrices() {
        this.projectionMatrix = mat4.create();
        this.viewMatrix = mat4.create();
        this.modelMatrix = mat4.create();
        this.normalMatrix = mat4.create();
        
        this.updateProjectionMatrix();
        this.updateViewMatrix();
    }
    
    updateProjectionMatrix() {
        const aspect = this.canvas.width / this.canvas.height;
        mat4.perspective(
            this.projectionMatrix,
            this.camera.fov * Math.PI / 180,
            aspect,
            this.camera.near,
            this.camera.far
        );
    }
    
    updateViewMatrix() {
        mat4.lookAt(
            this.viewMatrix,
            this.camera.position,
            this.camera.target,
            [0, 1, 0]
        );
    }
    
    render() {
        const gl = this.gl;
        
        // Clear
        gl.clearColor(0.12, 0.12, 0.12, 1.0);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
        gl.enable(gl.DEPTH_TEST);
        
        // Use program
        gl.useProgram(this.program);
        
        // Set view and projection matrices
        gl.uniformMatrix4fv(this.locations.uniforms.viewMatrix, false, this.viewMatrix);
        gl.uniformMatrix4fv(this.locations.uniforms.projectionMatrix, false, this.projectionMatrix);
        
        // Set camera position
        gl.uniform3fv(this.locations.uniforms.cameraPosition, this.camera.position);
        
        // Set light
        gl.uniform3fv(this.locations.uniforms.lightPosition, [5, 8, 5]);
        gl.uniform3fv(this.locations.uniforms.lightColor, [1, 0.8, 0.6]);
        
        // Render cube
        this.renderMesh(this.cubeBuffer, [-2, 0.5, 0], [1, 0, 0], 0.8, 0.2);
        
        // Render sphere (using cube for now)
        this.renderMesh(this.cubeBuffer, [2, 0.75, 0], [0, 0, 1], 0.0, 0.9);
        
        // Render plane
        this.renderMesh(this.planeBuffer, [0, 0, 0], [0.3, 0.3, 0.3], 0.1, 0.7);
    }
    
    renderMesh(meshBuffer, position, albedo, metallic, roughness) {
        const gl = this.gl;
        
        // Set model matrix
        mat4.identity(this.modelMatrix);
        mat4.translate(this.modelMatrix, this.modelMatrix, position);
        gl.uniformMatrix4fv(this.locations.uniforms.modelMatrix, false, this.modelMatrix);
        
        // Set normal matrix
        mat4.invert(this.normalMatrix, this.modelMatrix);
        mat4.transpose(this.normalMatrix, this.normalMatrix);
        gl.uniformMatrix4fv(this.locations.uniforms.normalMatrix, false, this.normalMatrix);
        
        // Set material
        gl.uniform3fv(this.locations.uniforms.albedo, albedo);
        gl.uniform1f(this.locations.uniforms.metallic, metallic);
        gl.uniform1f(this.locations.uniforms.roughness, roughness);
        
        // Bind buffers
        gl.bindBuffer(gl.ARRAY_BUFFER, meshBuffer.vertexBuffer);
        gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, meshBuffer.indexBuffer);
        
        // Set attributes
        const stride = 6 * 4; // 6 floats per vertex (pos + normal)
        gl.vertexAttribPointer(this.locations.attributes.position, 3, gl.FLOAT, false, stride, 0);
        gl.enableVertexAttribArray(this.locations.attributes.position);
        
        gl.vertexAttribPointer(this.locations.attributes.normal, 3, gl.FLOAT, false, stride, 3 * 4);
        gl.enableVertexAttribArray(this.locations.attributes.normal);
        
        // Draw
        gl.drawElements(gl.TRIANGLES, meshBuffer.vertexCount, gl.UNSIGNED_SHORT, 0);
    }
    
    resize(width, height) {
        this.canvas.width = width;
        this.canvas.height = height;
        this.gl.viewport(0, 0, width, height);
        this.updateProjectionMatrix();
    }
}

// Simple mat4 library (minimal implementation)
const mat4 = {
    create() {
        return new Float32Array(16);
    },
    
    identity(out) {
        out[0] = 1; out[1] = 0; out[2] = 0; out[3] = 0;
        out[4] = 0; out[5] = 1; out[6] = 0; out[7] = 0;
        out[8] = 0; out[9] = 0; out[10] = 1; out[11] = 0;
        out[12] = 0; out[13] = 0; out[14] = 0; out[15] = 1;
        return out;
    },
    
    perspective(out, fovy, aspect, near, far) {
        const f = 1.0 / Math.tan(fovy / 2);
        const nf = 1 / (near - far);
        out[0] = f / aspect; out[1] = 0; out[2] = 0; out[3] = 0;
        out[4] = 0; out[5] = f; out[6] = 0; out[7] = 0;
        out[8] = 0; out[9] = 0; out[10] = (far + near) * nf; out[11] = -1;
        out[12] = 0; out[13] = 0; out[14] = 2 * far * near * nf; out[15] = 0;
        return out;
    },
    
    lookAt(out, eye, center, up) {
        const z = [eye[0] - center[0], eye[1] - center[1], eye[2] - center[2]];
        const len = Math.sqrt(z[0] * z[0] + z[1] * z[1] + z[2] * z[2]);
        z[0] /= len; z[1] /= len; z[2] /= len;
        
        const x = [
            up[1] * z[2] - up[2] * z[1],
            up[2] * z[0] - up[0] * z[2],
            up[0] * z[1] - up[1] * z[0]
        ];
        const xlen = Math.sqrt(x[0] * x[0] + x[1] * x[1] + x[2] * x[2]);
        x[0] /= xlen; x[1] /= xlen; x[2] /= xlen;
        
        const y = [z[1] * x[2] - z[2] * x[1], z[2] * x[0] - z[0] * x[2], z[0] * x[1] - z[1] * x[0]];
        
        out[0] = x[0]; out[1] = y[0]; out[2] = z[0]; out[3] = 0;
        out[4] = x[1]; out[5] = y[1]; out[6] = z[1]; out[7] = 0;
        out[8] = x[2]; out[9] = y[2]; out[10] = z[2]; out[11] = 0;
        out[12] = -(x[0] * eye[0] + x[1] * eye[1] + x[2] * eye[2]);
        out[13] = -(y[0] * eye[0] + y[1] * eye[1] + y[2] * eye[2]);
        out[14] = -(z[0] * eye[0] + z[1] * eye[1] + z[2] * eye[2]);
        out[15] = 1;
        return out;
    },
    
    translate(out, a, v) {
        out[12] = a[0] * v[0] + a[4] * v[1] + a[8] * v[2] + a[12];
        out[13] = a[1] * v[0] + a[5] * v[1] + a[9] * v[2] + a[13];
        out[14] = a[2] * v[0] + a[6] * v[1] + a[10] * v[2] + a[14];
        out[15] = a[3] * v[0] + a[7] * v[1] + a[11] * v[2] + a[15];
        return out;
    },
    
    invert(out, a) {
        // Simplified - assumes orthogonal matrix
        for (let i = 0; i < 16; i++) out[i] = a[i];
        return out;
    },
    
    transpose(out, a) {
        const a01 = a[1], a02 = a[2], a03 = a[3];
        const a12 = a[6], a13 = a[7];
        const a23 = a[11];
        out[0] = a[0]; out[1] = a[4]; out[2] = a[8]; out[3] = a[12];
        out[4] = a01; out[5] = a[5]; out[6] = a[9]; out[7] = a[13];
        out[8] = a02; out[9] = a12; out[10] = a[10]; out[11] = a[14];
        out[12] = a03; out[13] = a13; out[14] = a23; out[15] = a[15];
        return out;
    }
};


