use super::*;

#[test]
fn paint_text_converts_compares_and_shares_storage() {
    let text: PaintText = String::from("Tempo").into();
    let cloned = text.clone();

    assert_eq!(text.as_str(), "Tempo");
    assert_eq!(text, "Tempo");
    assert_eq!("Tempo", cloned);
    assert!(Arc::ptr_eq(&text.0, &cloned.0));
}
