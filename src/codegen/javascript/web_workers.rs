//! Web Worker generation for browser parallelism
//!
//! Translates Windjammer's `spawn` keyword to Web Workers for browser environments.

use crate::parser::Statement;

/// Web Worker generator
pub struct WebWorkerGenerator {
    worker_count: usize,
}

impl WebWorkerGenerator {
    /// Create a new Web Worker generator
    pub fn new() -> Self {
        Self { worker_count: 0 }
    }

    /// Check if a statement uses spawn
    pub fn contains_spawn(stmt: &Statement) -> bool {
        match stmt {
            Statement::Go { .. } => true,
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                then_block.iter().any(Self::contains_spawn)
                    || else_block
                        .as_ref()
                        .is_some_and(|b| b.iter().any(Self::contains_spawn))
            }
            _ => false,
        }
    }

    /// Generate Web Worker code for a spawn statement
    pub fn generate_worker(&mut self, _body: &[Statement]) -> String {
        self.worker_count += 1;
        let worker_id = self.worker_count;

        let mut output = String::new();

        // Generate the worker function
        output.push_str(&format!(
            "// Worker {}\nconst worker{} = new Worker(",
            worker_id, worker_id
        ));

        // Create inline worker using Blob
        output.push_str("URL.createObjectURL(new Blob([`\n");
        output.push_str("self.onmessage = function(e) {\n");
        output.push_str("    try {\n");
        output.push_str("        // Worker code\n");

        // TODO: Generate actual worker body from statements
        output.push_str("        const result = (function() {\n");
        output.push_str("            // Execute spawned code\n");
        output.push_str("        })();\n");

        output.push_str("        self.postMessage({ success: true, result });\n");
        output.push_str("    } catch (error) {\n");
        output.push_str("        self.postMessage({ success: false, error: error.message });\n");
        output.push_str("    }\n");
        output.push_str("};\n");
        output.push_str("`], { type: 'application/javascript' })));\n\n");

        // Post message to worker
        output.push_str(&format!(
            "worker{}.postMessage({{}}); // Start worker\n",
            worker_id
        ));

        output
    }

    /// Generate helper code for Web Worker support
    pub fn generate_helpers() -> String {
        r#"
// Web Worker Helper Functions
function createWorkerFromFunction(fn) {
    const blob = new Blob([`
        self.onmessage = function(e) {
            const fn = ${fn.toString()};
            try {
                const result = fn(e.data);
                self.postMessage({ success: true, result });
            } catch (error) {
                self.postMessage({ success: false, error: error.message });
            }
        };
    `], { type: 'application/javascript' });
    
    return new Worker(URL.createObjectURL(blob));
}

function spawnWorker(fn, data) {
    return new Promise((resolve, reject) => {
        const worker = createWorkerFromFunction(fn);
        
        worker.onmessage = function(e) {
            worker.terminate();
            if (e.data.success) {
                resolve(e.data.result);
            } else {
                reject(new Error(e.data.error));
            }
        };
        
        worker.onerror = function(error) {
            worker.terminate();
            reject(error);
        };
        
        worker.postMessage(data);
    });
}

// Channel simulation using SharedArrayBuffer (if available) or MessageChannel
class Channel {
    constructor() {
        if (typeof SharedArrayBuffer !== 'undefined') {
            // Use SharedArrayBuffer for true shared memory
            this.buffer = new SharedArrayBuffer(1024);
            this.view = new Int32Array(this.buffer);
            this.writeIndex = 0;
            this.readIndex = 0;
        } else {
            // Fallback to message passing
            this.messageChannel = new MessageChannel();
            this.queue = [];
        }
    }
    
    send(value) {
        if (this.messageChannel) {
            this.messageChannel.port1.postMessage(value);
        } else {
            // SharedArrayBuffer implementation
            const json = JSON.stringify(value);
            // Write to shared memory (simplified)
        }
    }
    
    async recv() {
        if (this.messageChannel) {
            return new Promise((resolve) => {
                this.messageChannel.port2.onmessage = (e) => {
                    resolve(e.data);
                };
            });
        } else {
            // SharedArrayBuffer implementation
            return new Promise((resolve) => {
                // Read from shared memory (simplified)
            });
        }
    }
}

function createChannel() {
    return new Channel();
}
"#
        .to_string()
    }
}

impl Default for WebWorkerGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Transform spawn statements to Web Workers
pub fn transform_spawn_to_workers(statements: &[Statement]) -> String {
    let mut generator = WebWorkerGenerator::new();
    let mut output = String::new();

    // Add helpers at the top
    output.push_str(&WebWorkerGenerator::generate_helpers());
    output.push('\n');

    for stmt in statements {
        if let Statement::Go { body } = stmt {
            output.push_str(&generator.generate_worker(body));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Expression;

    #[test]
    fn test_generate_helpers() {
        let helpers = WebWorkerGenerator::generate_helpers();
        assert!(helpers.contains("createWorkerFromFunction"));
        assert!(helpers.contains("spawnWorker"));
        assert!(helpers.contains("Channel"));
    }

    #[test]
    fn test_web_worker_generator() {
        let mut generator = WebWorkerGenerator::new();
        let worker_code = generator.generate_worker(&[]);

        assert!(worker_code.contains("new Worker"));
        assert!(worker_code.contains("Blob"));
        assert!(worker_code.contains("onmessage"));
    }

    #[test]
    fn test_contains_spawn() {
        let spawn_stmt = Statement::Go { body: vec![] };

        assert!(WebWorkerGenerator::contains_spawn(&spawn_stmt));

        let regular_stmt =
            Statement::Expression(Expression::Literal(crate::parser::Literal::Int(42)));

        assert!(!WebWorkerGenerator::contains_spawn(&regular_stmt));
    }
}
