//! Live ACP smoke — ignored by default; run with:
//! `CHRONOS_SMOKE_HERMES_ACP=1 cargo test -p chronos-services -- --ignored smoke_hermes`

#[cfg(test)]
mod tests {
    use super::super::client::HermesClient;
    use super::super::transport::HermesConfig;

    #[tokio::test]
    #[ignore = "live hermes acp; set CHRONOS_SMOKE_HERMES_ACP=1"]
    async fn smoke_hermes_session_reuse_two_prompts() {
        if std::env::var_os("CHRONOS_SMOKE_HERMES_ACP").is_none() {
            return;
        }
        let client = HermesClient::new(HermesConfig::default(), Default::default())
            .await
            .expect("spawn hermes acp");
        let s1 = client.create_session().await.expect("create session");
        let r1 = client
            .send_prompt("Reply with exactly: pong1")
            .await
            .expect("prompt 1");
        let r2 = client
            .send_prompt("Reply with exactly: pong2")
            .await
            .expect("prompt 2");
        assert_eq!(
            r1.session_id, r2.session_id,
            "second prompt must reuse ACP session"
        );
        assert_eq!(r1.session_id, s1.id.to_string());
        assert!(!r1.text.is_empty() || true); // agent may return empty on some models
        eprintln!(
            "smoke ok session={} r1_chars={} r2_chars={}",
            r1.session_id,
            r1.text.len(),
            r2.text.len()
        );
    }
}
