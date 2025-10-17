//! Polyfills for older browser support
//!
//! Provides polyfills for modern JavaScript features to support older browsers.

/// Polyfill configuration
#[derive(Debug, Clone)]
pub struct PolyfillConfig {
    /// Target browser/environment
    pub target: PolyfillTarget,
    /// Include Promise polyfill
    pub include_promise: bool,
    /// Include Array methods polyfills
    pub include_array_methods: bool,
    /// Include Object methods polyfills
    pub include_object_methods: bool,
    /// Include Symbol polyfill
    pub include_symbol: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyfillTarget {
    /// ES5 (IE9+)
    ES5,
    /// ES2015/ES6 (IE11+)
    ES2015,
    /// ES2017 (Modern browsers)
    ES2017,
    /// ES2020+ (Latest)
    ES2020,
}

impl Default for PolyfillConfig {
    fn default() -> Self {
        Self {
            target: PolyfillTarget::ES2015,
            include_promise: true,
            include_array_methods: true,
            include_object_methods: true,
            include_symbol: false,
        }
    }
}

/// Generate polyfills based on configuration
pub fn generate_polyfills(config: &PolyfillConfig) -> String {
    let mut polyfills = String::new();

    polyfills.push_str("// Windjammer Polyfills\n");
    polyfills.push_str("(function(global) {\n");
    polyfills.push_str("  'use strict';\n\n");

    if config.include_promise {
        polyfills.push_str(&promise_polyfill());
    }

    if config.include_array_methods {
        polyfills.push_str(&array_methods_polyfill(config.target));
    }

    if config.include_object_methods {
        polyfills.push_str(&object_methods_polyfill(config.target));
    }

    if config.include_symbol {
        polyfills.push_str(&symbol_polyfill());
    }

    polyfills.push_str("})(typeof window !== 'undefined' ? window : global);\n\n");

    polyfills
}

/// Promise polyfill for older browsers
fn promise_polyfill() -> String {
    r#"  // Promise polyfill
  if (typeof Promise === 'undefined') {
    global.Promise = function(executor) {
      var state = 'pending';
      var value;
      var handlers = [];
      
      function resolve(result) {
        if (state !== 'pending') return;
        state = 'fulfilled';
        value = result;
        handlers.forEach(handle);
      }
      
      function reject(error) {
        if (state !== 'pending') return;
        state = 'rejected';
        value = error;
        handlers.forEach(handle);
      }
      
      function handle(handler) {
        if (state === 'pending') {
          handlers.push(handler);
        } else {
          setTimeout(function() {
            var callback = state === 'fulfilled' ? handler.onFulfilled : handler.onRejected;
            if (callback) {
              try {
                handler.resolve(callback(value));
              } catch (e) {
                handler.reject(e);
              }
            } else {
              (state === 'fulfilled' ? handler.resolve : handler.reject)(value);
            }
          }, 0);
        }
      }
      
      this.then = function(onFulfilled, onRejected) {
        return new Promise(function(resolve, reject) {
          handle({
            onFulfilled: onFulfilled,
            onRejected: onRejected,
            resolve: resolve,
            reject: reject
          });
        });
      };
      
      this.catch = function(onRejected) {
        return this.then(null, onRejected);
      };
      
      try {
        executor(resolve, reject);
      } catch (e) {
        reject(e);
      }
    };
    
    Promise.resolve = function(value) {
      return new Promise(function(resolve) {
        resolve(value);
      });
    };
    
    Promise.reject = function(error) {
      return new Promise(function(_, reject) {
        reject(error);
      });
    };
    
    Promise.all = function(promises) {
      return new Promise(function(resolve, reject) {
        var results = [];
        var remaining = promises.length;
        
        if (remaining === 0) {
          resolve(results);
          return;
        }
        
        promises.forEach(function(promise, index) {
          Promise.resolve(promise).then(function(value) {
            results[index] = value;
            remaining--;
            if (remaining === 0) {
              resolve(results);
            }
          }, reject);
        });
      });
    };
  }

"#
    .to_string()
}

/// Array methods polyfill
fn array_methods_polyfill(target: PolyfillTarget) -> String {
    let mut polyfills = String::new();

    polyfills.push_str("  // Array methods polyfill\n");

    // Array.from (ES6)
    if target as u8 <= PolyfillTarget::ES2015 as u8 {
        polyfills.push_str(
            r#"  if (!Array.from) {
    Array.from = function(arrayLike) {
      return Array.prototype.slice.call(arrayLike);
    };
  }

"#,
        );
    }

    // Array.prototype.find (ES6)
    if target as u8 <= PolyfillTarget::ES2015 as u8 {
        polyfills.push_str(
            r#"  if (!Array.prototype.find) {
    Array.prototype.find = function(predicate) {
      for (var i = 0; i < this.length; i++) {
        if (predicate(this[i], i, this)) {
          return this[i];
        }
      }
    };
  }

"#,
        );
    }

    // Array.prototype.includes (ES7)
    if target as u8 <= PolyfillTarget::ES2017 as u8 {
        polyfills.push_str(
            r#"  if (!Array.prototype.includes) {
    Array.prototype.includes = function(element) {
      return this.indexOf(element) !== -1;
    };
  }

"#,
        );
    }

    polyfills
}

/// Object methods polyfill
fn object_methods_polyfill(target: PolyfillTarget) -> String {
    let mut polyfills = String::new();

    polyfills.push_str("  // Object methods polyfill\n");

    // Object.assign (ES6)
    if target as u8 <= PolyfillTarget::ES2015 as u8 {
        polyfills.push_str(
            r#"  if (!Object.assign) {
    Object.assign = function(target) {
      for (var i = 1; i < arguments.length; i++) {
        var source = arguments[i];
        for (var key in source) {
          if (Object.prototype.hasOwnProperty.call(source, key)) {
            target[key] = source[key];
          }
        }
      }
      return target;
    };
  }

"#,
        );
    }

    // Object.values (ES8)
    if target as u8 <= PolyfillTarget::ES2017 as u8 {
        polyfills.push_str(
            r#"  if (!Object.values) {
    Object.values = function(obj) {
      var values = [];
      for (var key in obj) {
        if (Object.prototype.hasOwnProperty.call(obj, key)) {
          values.push(obj[key]);
        }
      }
      return values;
    };
  }

"#,
        );
    }

    polyfills
}

/// Symbol polyfill
fn symbol_polyfill() -> String {
    r#"  // Symbol polyfill
  if (typeof Symbol === 'undefined') {
    var symbolCounter = 0;
    global.Symbol = function(description) {
      var symbol = '__symbol_' + (symbolCounter++) + '_' + (description || '');
      return symbol;
    };
    
    Symbol.for = function(key) {
      return '__symbol_for_' + key;
    };
  }

"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_polyfills_default() {
        let config = PolyfillConfig::default();
        let polyfills = generate_polyfills(&config);

        assert!(polyfills.contains("Promise"));
        assert!(polyfills.contains("Array"));
        assert!(polyfills.contains("Object"));
    }

    #[test]
    fn test_generate_polyfills_es5() {
        let config = PolyfillConfig {
            target: PolyfillTarget::ES5,
            ..Default::default()
        };
        let polyfills = generate_polyfills(&config);

        assert!(polyfills.contains("Promise"));
        assert!(polyfills.contains("Array.from"));
        assert!(polyfills.contains("Object.assign"));
    }

    #[test]
    fn test_generate_polyfills_minimal() {
        let config = PolyfillConfig {
            target: PolyfillTarget::ES2020,
            include_promise: false,
            include_array_methods: false,
            include_object_methods: false,
            include_symbol: false,
        };
        let polyfills = generate_polyfills(&config);

        // Should have minimal content
        assert!(polyfills.contains("Windjammer Polyfills"));
    }

    #[test]
    fn test_promise_polyfill() {
        let polyfill = promise_polyfill();
        assert!(polyfill.contains("Promise"));
        assert!(polyfill.contains("resolve"));
        assert!(polyfill.contains("reject"));
        assert!(polyfill.contains("then"));
        assert!(polyfill.contains("catch"));
    }
}
