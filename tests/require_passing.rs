// use lazy_static::lazy_static;
// use regex::Regex;
// use sieve_language_server::datastructures::*;

// #[test]
// fn test_require_parsing() {
//     // Create a mock client for testing
//     // use tower_lsp::lsp_types::*;
//     // use tower_lsp::Client;

//     // We can't easily create a real Client for testing, so we'll test
//     // the parsing logic in isolation by creating a test implementation
//     struct TestSieveParser;

//     impl TestSieveParser {
//         fn parse_require_statement(&self, line: &str) -> Option<Vec<String>> {
//             // Copy the implementation from SieveLanguageServer
//             lazy_static! {
//                 static ref REQUIRE_REGEX: Regex =
//                     Regex::new(r#"require\s+(?:\[([^\]]+)\]|"([^"]+)")"#).unwrap();
//             }

//             if let Some(captures) = REQUIRE_REGEX.captures(line) {
//                 if let Some(list_match) = captures.get(1) {
//                     // Handle array format: require ["ext1", "ext2"];
//                     let list_content = list_match.as_str();
//                     let extensions: Vec<String> = list_content
//                         .split(',')
//                         .map(|s| s.trim().trim_matches('"').to_string())
//                         .collect();
//                     Some(extensions)
//                 } else if let Some(single_match) = captures.get(2) {
//                     // Handle single string format: require "ext";
//                     Some(vec![single_match.as_str().to_string()])
//                 } else {
//                     None
//                 }
//             } else {
//                 None
//             }
//         }
//     }

//     let parser = TestSieveParser;

//     // Test single extension
//     let result = parser.parse_require_statement("require \"fileinto\";");
//     assert_eq!(result, Some(vec!["fileinto".to_string()]));

//     // Test multiple extensions
//     let result = parser.parse_require_statement("require [\"body\", \"regex\"];");
//     assert_eq!(result, Some(vec!["body".to_string(), "regex".to_string()]));
// }
