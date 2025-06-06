use sieve_language_server::sieve::{SIEVE_ACTIONS, SIEVE_EXTENSIONS, SIEVE_TAGS, SIEVE_TESTS};

#[test]
fn test_sieve_definitions() {
    // Test that our Sieve definitions are not empty
    assert!(!SIEVE_TESTS.is_empty());
    assert!(!SIEVE_ACTIONS.is_empty());
    assert!(!SIEVE_TAGS.is_empty());
    assert!(!SIEVE_EXTENSIONS.is_empty());

    // Test that specific commands exist
    assert!(SIEVE_TESTS.contains(&"header"));
    assert!(SIEVE_ACTIONS.contains(&"fileinto"));
    assert!(SIEVE_TAGS.contains(&":contains"));
}
