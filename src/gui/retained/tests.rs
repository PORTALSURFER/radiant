use super::RetainedVec;

#[test]
fn retained_vec_clones_share_storage_until_mutation() {
    let mut original = RetainedVec::from(vec![1, 2, 3]);
    let clone = original.clone();

    original.push(4);

    assert_eq!(clone.as_slice(), &[1, 2, 3]);
    assert_eq!(original.as_slice(), &[1, 2, 3, 4]);
}
