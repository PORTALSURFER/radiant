use super::*;

#[test]
fn paint_text_converts_compares_and_shares_storage() {
    let text: PaintText = String::from("Tempo").into();
    let cloned = text.clone();

    assert_eq!(text.as_str(), "Tempo");
    assert_eq!(text, "Tempo");
    assert_eq!("Tempo", cloned);
    let PaintTextStorage::Shared(text) = &text.0 else {
        panic!("owned text should use shared storage");
    };
    let PaintTextStorage::Shared(cloned) = &cloned.0 else {
        panic!("cloned owned text should use shared storage");
    };
    assert!(Arc::ptr_eq(text, cloned));
}

#[test]
fn paint_text_preserves_static_and_existing_shared_storage() {
    let static_text = PaintText::from_static("Ready");
    assert!(static_text.is_static());
    assert_eq!(static_text, "Ready");

    let shared: Arc<str> = Arc::from("Loading");
    let text = PaintText::from(Arc::clone(&shared));
    let PaintTextStorage::Shared(stored) = &text.0 else {
        panic!("Arc text should preserve shared storage");
    };
    assert!(Arc::ptr_eq(&shared, stored));
}
