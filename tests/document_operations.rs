use sieve_language_server::datastructures::*;
use url::Url;

#[test]
fn test_document_operations() {
    let uri = Url::parse("file:///test.sieve").unwrap();
    let doc = SieveDocument::new(uri, "test content".to_string(), 1);

    // Test basic operations
    assert_eq!(doc.get_text(), "test content");
    assert_eq!(doc.version, 1);

    // Test getting lines
    let uri2 = Url::parse("file:///test2.sieve").unwrap();
    let doc2 = SieveDocument::new(uri2, "line1\nline2\nline3".to_string(), 1);
    assert_eq!(doc2.get_line(0), Some("line1\n".to_string()));
    assert_eq!(doc2.get_line(1), Some("line2\n".to_string()));
    assert_eq!(doc2.get_line(10), None);
}
