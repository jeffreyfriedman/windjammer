// Windjammer std::json backend for Go (encoding/json).
// API mirrors windjammer_runtime::json (Rust).
package wjjson

import "encoding/json"

func Parse(s string) (map[string]interface{}, error) {
	var v map[string]interface{}
	if err := json.Unmarshal([]byte(s), &v); err != nil {
		return nil, err
	}
	return v, nil
}

func Stringify(v interface{}) (string, error) {
	b, err := json.Marshal(v)
	if err != nil {
		return "", err
	}
	return string(b), nil
}
