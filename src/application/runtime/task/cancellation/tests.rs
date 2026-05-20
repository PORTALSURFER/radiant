use super::CancellationToken;

#[test]
fn cancellation_token_is_shared_across_clones() {
    let token = CancellationToken::new();
    let worker_token = token.clone();

    assert!(!worker_token.is_cancelled());
    token.cancel();

    assert!(worker_token.is_cancelled());
}
