#[test]
fn tdd_ratchet_gatekeeper() {
    if std::env::var("TDD_RATCHET").is_err() {
        panic!(
            "This project uses tdd-ratchet for strict TDD.\n\
             Run `cargo ratchet` instead of `cargo test`."
        );
    }
}
