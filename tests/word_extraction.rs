// use sieve_language_server::datastructures::*;

// #[test]
// fn test_word_extraction() {
//     // Create a test helper that mimics the word extraction logic
//     fn get_word_at_position(line: &str, character: usize) -> Option<String> {
//         if character > line.len() {
//             return None;
//         }

//         let chars: Vec<char> = line.chars().collect();
//         let mut start = character;
//         let mut end = character;

//         // Find start of word
//         while start > 0
//             && (chars[start - 1].is_alphanumeric()
//                 || chars[start - 1] == '_'
//                 || chars[start - 1] == ':')
//         {
//             start -= 1;
//         }

//         // Find end of word
//         while end < chars.len()
//             && (chars[end].is_alphanumeric() || chars[end] == '_' || chars[end] == ':')
//         {
//             end += 1;
//         }

//         if start < end {
//             Some(chars[start..end].iter().collect())
//         } else {
//             None
//         }
//     }

//     // Test word extraction
//     let line = "if header :contains \"from\" \"test\"";

//     // Test extracting "header"
//     assert_eq!(get_word_at_position(line, 3), Some("header".to_string()));
//     assert_eq!(get_word_at_position(line, 6), Some("header".to_string()));

//     // Test extracting ":contains"
//     assert_eq!(
//         get_word_at_position(line, 10),
//         Some(":contains".to_string())
//     );
//     assert_eq!(
//         get_word_at_position(line, 15),
//         Some(":contains".to_string())
//     );

//     // Test edge cases
//     assert_eq!(get_word_at_position(line, 0), Some("if".to_string()));
//     assert_eq!(get_word_at_position(line, 100), None); // Out of bounds
//     assert_eq!(get_word_at_position("", 0), None); // Empty string
// }
