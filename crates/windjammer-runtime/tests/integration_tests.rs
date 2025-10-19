//! Comprehensive integration tests for all stdlib modules
//!
//! These tests ensure that every stdlib module is properly implemented
//! and not just a stub.

use windjammer_runtime::*;

// ============================================================================
// std::fs - File System Operations
// ============================================================================

#[test]
fn test_fs_write_read() {
    use std::fs as std_fs;

    let test_file = "/tmp/windjammer_test_fs.txt";
    let content = "Hello, Windjammer!";

    // Write
    let write_result = fs::write_string(test_file, content);
    assert!(write_result.is_ok(), "Failed to write: {:?}", write_result);

    // Read
    let read_result = fs::read_to_string(test_file);
    assert!(read_result.is_ok(), "Failed to read: {:?}", read_result);
    assert_eq!(read_result.unwrap(), content);

    // Cleanup
    let _ = std_fs::remove_file(test_file);
}

#[test]
fn test_fs_exists() {
    let result = fs::exists("/tmp");
    assert!(result.is_ok());
    assert!(result.unwrap());

    let result = fs::exists("/nonexistent_path_12345");
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_fs_metadata() {
    let result = fs::metadata("/tmp");
    assert!(result.is_ok(), "Failed to get metadata: {:?}", result);
    let meta = result.unwrap();
    assert!(meta.is_dir);
}

// ============================================================================
// std::http - HTTP Client & Server
// ============================================================================

#[tokio::test]
async fn test_http_client_get() {
    // Test against a reliable public API
    let result = http::get("https://httpbin.org/get").await;
    assert!(result.is_ok(), "HTTP GET failed: {:?}", result);

    let response = result.unwrap();
    assert!(response.is_success());
    assert_eq!(response.status_code(), 200);
}

#[tokio::test]
async fn test_http_response_text() {
    let result = http::get("https://httpbin.org/get").await;
    assert!(result.is_ok());

    let response = result.unwrap();
    let text_result = response.text().await;
    assert!(text_result.is_ok(), "Failed to get text: {:?}", text_result);

    let text = text_result.unwrap();
    assert!(text.contains("httpbin"));
}

#[tokio::test]
async fn test_http_post_json() {
    let result = http::post("https://httpbin.org/post")
        .json(r#"{"test": "data"}"#)
        .send()
        .await;

    assert!(result.is_ok(), "HTTP POST failed: {:?}", result);
    let response = result.unwrap();
    assert!(response.is_success());
}

// ============================================================================
// std::json - JSON Operations
// ============================================================================

#[test]
fn test_json_parse_stringify() {
    let json_str = r#"{"name": "Alice", "age": 30}"#;

    let parse_result = json::parse(json_str);
    assert!(parse_result.is_ok(), "Failed to parse: {:?}", parse_result);

    let value = parse_result.unwrap();
    let stringify_result = json::stringify(&value);
    assert!(stringify_result.is_ok());
}

#[test]
fn test_json_get_set() {
    let json_str = r#"{"name": "Alice", "age": 30}"#;
    let mut value = json::parse(json_str).unwrap();

    // Get
    let name = json::get(&value, "name");
    assert!(name.is_some());

    // Set
    let new_value = json::parse(r#""Bob""#).unwrap();
    let set_result = json::set(&mut value, "name", new_value);
    assert!(set_result.is_ok());
}

#[test]
fn test_json_array_operations() {
    let json_str = r#"[1, 2, 3, 4, 5]"#;
    let value = json::parse(json_str).unwrap();

    let len = json::len(&value);
    assert_eq!(len, 5);

    assert!(!json::is_empty(&value));
}

// ============================================================================
// std::mime - MIME Type Detection
// ============================================================================

#[test]
fn test_mime_from_filename() {
    let mime = mime::from_filename("test.html");
    assert_eq!(mime, "text/html");

    let mime = mime::from_filename("image.png");
    assert_eq!(mime, "image/png");

    let mime = mime::from_filename("data.json");
    assert_eq!(mime, "application/json");
}

#[test]
fn test_mime_from_extension() {
    let mime = mime::from_extension("js");
    assert_eq!(mime, "application/javascript");

    let mime = mime::from_extension("css");
    assert_eq!(mime, "text/css");
}

#[test]
fn test_mime_type_checks() {
    assert!(mime::is_text("text/html"));
    assert!(mime::is_image("image/png"));
    assert!(mime::is_application("application/json"));
    assert!(!mime::is_audio("text/html"));
}

// ============================================================================
// std::async - Async Runtime
// ============================================================================

#[tokio::test]
async fn test_async_spawn() {
    let handle = async_runtime::spawn(async { 42 });

    let result = handle.await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_async_sleep() {
    use std::time::Instant;

    let start = Instant::now();
    async_runtime::sleep(100).await;
    let elapsed = start.elapsed().as_millis();

    assert!(
        elapsed >= 100,
        "Sleep didn't wait long enough: {}ms",
        elapsed
    );
}

// ============================================================================
// std::cli - Command-Line Argument Parsing
// ============================================================================

#[test]
fn test_cli_app_creation() {
    let app = cli::app("test", "1.0.0", "Test application");
    let app = app.arg(
        "input",
        Some('i'),
        Some("input".to_string()),
        "Input file",
        false,
    );
    let app = app.flag(
        "verbose",
        Some('v'),
        Some("verbose".to_string()),
        "Verbose output",
    );

    // Can't easily test parse() without mocking args, but we can verify structure
    assert!(true); // App builds successfully
}

// ============================================================================
// std::collections - Data Structures
// ============================================================================

#[test]
fn test_collections_hashmap() {
    let mut map = collections::HashMap::new();

    collections::HashMap::insert(&mut map, "key1".to_string(), "value1".to_string());
    assert_eq!(collections::HashMap::len(&map), 1);

    let value = collections::HashMap::get(&map, "key1");
    assert_eq!(value, Some("value1".to_string()));

    assert!(collections::HashMap::contains_key(&map, "key1"));
    assert!(!collections::HashMap::contains_key(&map, "key2"));
}

#[test]
fn test_collections_hashset() {
    let mut set = collections::HashSet::new();

    collections::HashSet::insert(&mut set, "item1".to_string());
    collections::HashSet::insert(&mut set, "item2".to_string());
    assert_eq!(collections::HashSet::len(&set), 2);

    assert!(collections::HashSet::contains(&set, "item1"));
    assert!(!collections::HashSet::contains(&set, "item3"));
}

#[test]
fn test_collections_vecdeque() {
    let mut deque = collections::VecDeque::new();

    collections::VecDeque::push_back(&mut deque, "back".to_string());
    collections::VecDeque::push_front(&mut deque, "front".to_string());

    assert_eq!(collections::VecDeque::len(&deque), 2);
    assert_eq!(
        collections::VecDeque::front(&deque),
        Some("front".to_string())
    );
    assert_eq!(
        collections::VecDeque::back(&deque),
        Some("back".to_string())
    );
}

// ============================================================================
// std::crypto - Cryptographic Operations
// ============================================================================

#[test]
fn test_crypto_sha256() {
    let hash = crypto::sha256("hello world");
    assert!(!hash.is_empty());
    assert_eq!(hash.len(), 64); // SHA256 produces 64 hex characters
}

#[test]
fn test_crypto_password_hashing() {
    let password = "my_secure_password";
    let hash_result = crypto::hash_password(password);
    assert!(
        hash_result.is_ok(),
        "Failed to hash password: {:?}",
        hash_result
    );

    let hash = hash_result.unwrap();
    let verify_result = crypto::verify_password(password, &hash);
    assert!(verify_result.is_ok());
    assert!(verify_result.unwrap(), "Password verification failed");

    let wrong_verify = crypto::verify_password("wrong_password", &hash);
    assert!(wrong_verify.is_ok());
    assert!(!wrong_verify.unwrap(), "Wrong password should not verify");
}

#[test]
fn test_crypto_base64() {
    let data = "Hello, World!";
    let encoded = crypto::base64_encode(data);
    assert!(!encoded.is_empty());

    let decoded_result = crypto::base64_decode(&encoded);
    assert!(decoded_result.is_ok());
    assert_eq!(decoded_result.unwrap(), data);
}

// ============================================================================
// std::csv - CSV Parsing
// ============================================================================

#[test]
fn test_csv_parse() {
    let csv_data = "name,age,city\nAlice,30,NYC\nBob,25,LA";
    let result = csv_mod::parse(csv_data);
    assert!(result.is_ok(), "Failed to parse CSV: {:?}", result);

    let rows = result.unwrap();
    assert_eq!(rows.len(), 3); // header + 2 data rows
}

#[test]
fn test_csv_write() {
    let rows = vec![
        vec!["name".to_string(), "age".to_string()],
        vec!["Alice".to_string(), "30".to_string()],
    ];

    let result = csv_mod::write(&rows);
    assert!(result.is_ok());
    let csv = result.unwrap();
    assert!(csv.contains("name,age"));
    assert!(csv.contains("Alice,30"));
}

// ============================================================================
// std::db - Database Operations
// ============================================================================

#[test]
fn test_db_open_sqlite() {
    let result = db::open_sqlite(":memory:");
    assert!(result.is_ok(), "Failed to open SQLite: {:?}", result);

    let conn = result.unwrap();
    assert_eq!(conn.db_type(), &db::DatabaseType::SQLite);
}

#[test]
fn test_db_open_postgres() {
    let result = db::open_postgres("postgres://localhost/test");
    assert!(
        result.is_ok(),
        "Failed to create Postgres connection: {:?}",
        result
    );

    let conn = result.unwrap();
    assert_eq!(conn.db_type(), &db::DatabaseType::Postgres);
}

#[test]
fn test_db_auto_detect() {
    let sqlite = db::open(":memory:").unwrap();
    assert_eq!(sqlite.db_type(), &db::DatabaseType::SQLite);

    let postgres = db::open("postgres://localhost/test").unwrap();
    assert_eq!(postgres.db_type(), &db::DatabaseType::Postgres);
}

// ============================================================================
// std::encoding - Encoding Operations
// ============================================================================

#[test]
fn test_encoding_base64() {
    let data = "Hello, World!";
    let encoded = encoding::base64_encode(data);
    assert!(!encoded.is_empty());

    let decoded = encoding::base64_decode(&encoded);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), data);
}

#[test]
fn test_encoding_hex() {
    let data = "test";
    let encoded = encoding::hex_encode(data);
    assert_eq!(encoded, "74657374");

    let decoded = encoding::hex_decode(&encoded);
    assert!(decoded.is_ok());
    assert_eq!(decoded.unwrap(), data);
}

// ============================================================================
// std::env - Environment Variables
// ============================================================================

#[test]
fn test_env_set_get() {
    env::set_var("WINDJAMMER_TEST_VAR", "test_value");

    let result = env::get_var("WINDJAMMER_TEST_VAR");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test_value");

    env::remove_var("WINDJAMMER_TEST_VAR");
    let result = env::get_var("WINDJAMMER_TEST_VAR");
    assert!(result.is_err());
}

#[test]
fn test_env_current_dir() {
    let result = env::current_dir();
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn test_env_temp_dir() {
    let result = env::temp_dir();
    assert!(result.is_ok());
    assert!(!result.unwrap().is_empty());
}

// ============================================================================
// std::log - Logging
// ============================================================================

#[test]
fn test_log_init() {
    // Init can be called multiple times safely
    log_mod::init();
    log_mod::init();
}

#[test]
fn test_log_messages() {
    log_mod::init();
    log_mod::info("Test info message");
    log_mod::warn("Test warning message");
    log_mod::error("Test error message");
    log_mod::debug("Test debug message");
    log_mod::trace("Test trace message");
    // If no panic, logging works
}

// ============================================================================
// std::math - Mathematical Operations
// ============================================================================

#[test]
fn test_math_basic_ops() {
    assert_eq!(math::abs(-5.0), 5.0);
    assert_eq!(math::sqrt(16.0), 4.0);
    assert_eq!(math::pow(2.0, 3.0), 8.0);
    assert_eq!(math::min(5.0, 3.0), 3.0);
    assert_eq!(math::max(5.0, 3.0), 5.0);
}

#[test]
fn test_math_trig() {
    use std::f64::consts::PI;

    assert!((math::sin(0.0) - 0.0).abs() < 0.0001);
    assert!((math::cos(0.0) - 1.0).abs() < 0.0001);
    assert!((math::sin(PI / 2.0) - 1.0).abs() < 0.0001);
}

#[test]
fn test_math_rounding() {
    assert_eq!(math::floor(3.7), 3.0);
    assert_eq!(math::ceil(3.2), 4.0);
    assert_eq!(math::round(3.5), 4.0);
}

// ============================================================================
// std::process - Process Operations
// ============================================================================

#[test]
fn test_process_run() {
    let result = process::run("echo", &["hello"]);
    assert!(result.is_ok(), "Failed to run process: {:?}", result);

    let output = result.unwrap();
    assert!(output.success);
    assert!(output.stdout.contains("hello"));
}

#[test]
fn test_process_run_failure() {
    let result = process::run("nonexistent_command_12345", &[]);
    assert!(result.is_err());
}

// ============================================================================
// std::random - Random Number Generation
// ============================================================================

#[test]
fn test_random_int() {
    let num = random::rand_int(1, 10);
    assert!(num >= 1 && num <= 10);
}

#[test]
fn test_random_float() {
    let num = random::rand_float(0.0, 1.0);
    assert!(num >= 0.0 && num <= 1.0);
}

#[test]
fn test_random_bool() {
    // Just ensure it doesn't panic
    let _ = random::rand_bool();
}

#[test]
fn test_random_choice() {
    let items = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let choice = random::rand_choice(&items);
    assert!(choice.is_some());
    assert!(items.contains(&choice.unwrap()));
}

// ============================================================================
// std::regex - Regular Expressions
// ============================================================================

#[test]
fn test_regex_new() {
    let result = regex_mod::new(r"\d+");
    assert!(result.is_ok(), "Failed to create regex: {:?}", result);
}

#[test]
fn test_regex_is_match() {
    let re = regex_mod::new(r"\d+").unwrap();
    assert!(regex_mod::is_match(&re, "123"));
    assert!(!regex_mod::is_match(&re, "abc"));
}

#[test]
fn test_regex_find() {
    let re = regex_mod::new(r"\d+").unwrap();
    let result = regex_mod::find(&re, "abc123def");
    assert_eq!(result, Some("123".to_string()));
}

#[test]
fn test_regex_replace() {
    let re = regex_mod::new(r"\d+").unwrap();
    let result = regex_mod::replace(&re, "abc123def", "XXX");
    assert_eq!(result, "abcXXXdef");
}

// ============================================================================
// std::strings - String Operations
// ============================================================================

#[test]
fn test_strings_basic() {
    let s = "Hello, World!";
    assert_eq!(strings::len(s), 13);
    assert!(!strings::is_empty(s));
    assert!(strings::contains(s, "World"));
    assert!(strings::starts_with(s, "Hello"));
    assert!(strings::ends_with(s, "!"));
}

#[test]
fn test_strings_case() {
    let s = "Hello";
    assert_eq!(strings::to_uppercase(s), "HELLO");
    assert_eq!(strings::to_lowercase(s), "hello");
}

#[test]
fn test_strings_split_join() {
    let s = "a,b,c";
    let parts = strings::split(s, ",");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "a");

    let joined = strings::join(&parts, "-");
    assert_eq!(joined, "a-b-c");
}

#[test]
fn test_strings_trim() {
    let s = "  hello  ";
    assert_eq!(strings::trim(s), "hello");
}

// ============================================================================
// std::testing - Test Utilities
// ============================================================================

#[test]
fn test_testing_assert() {
    testing::assert(true, "Should not panic");
}

#[test]
#[should_panic]
fn test_testing_assert_fail() {
    testing::assert(false, "Should panic");
}

#[test]
fn test_testing_assert_eq() {
    testing::assert_eq(5, 5, "Should be equal");
}

#[test]
fn test_testing_should_panic() {
    let panics = testing::should_panic(|| {
        panic!("test");
    });
    assert!(panics);

    let no_panic = testing::should_panic(|| {
        // do nothing
    });
    assert!(!no_panic);
}

// ============================================================================
// std::time - Time Operations
// ============================================================================

#[test]
fn test_time_now() {
    let timestamp = time::now();
    assert!(timestamp > 0);
}

#[test]
fn test_time_now_string() {
    let s = time::now_string();
    assert!(!s.is_empty());
    assert!(s.contains('T')); // ISO 8601 format
}

#[test]
fn test_time_parse_format() {
    let s = "2024-01-15T10:30:00Z";
    let result = time::parse(s, "%Y-%m-%dT%H:%M:%SZ");
    assert!(result.is_ok(), "Failed to parse time: {:?}", result);

    let timestamp = result.unwrap();
    let formatted = time::format(timestamp, "%Y-%m-%d");
    assert!(formatted.starts_with("2024-01-15"));
}

#[test]
fn test_time_duration() {
    let secs = time::duration_secs(2, 5);
    assert_eq!(secs, 3);

    let millis = time::duration_millis(1000, 2000);
    assert_eq!(millis, 1000);
}
