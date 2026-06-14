// Windjammer std::json backend for JavaScript (Node / browser).
export function parse(text) {
  return JSON.parse(text);
}

export function stringify(value) {
  return JSON.stringify(value);
}

export function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export function get(value, key) {
  if (value && typeof value === "object") return value[key];
  return undefined;
}
