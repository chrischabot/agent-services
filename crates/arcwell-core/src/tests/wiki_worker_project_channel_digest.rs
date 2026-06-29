use super::*;

mod candidates_wiki_backup;
mod channels_digest;
mod project_work_procedure;
mod research_source_worker;
mod watch_ingest_sources;

fn email_edge_payload(trusted_sender: &str, header_from: &str, message_id: &str) -> Value {
    json!({
        "provider": "cloudflare_email_routing",
        "messageId": message_id,
        "receivedAt": "2026-06-21T12:00:00Z",
        "routeId": "codex",
        "projectId": null,
        "trustedSender": trusted_sender,
        "headerFrom": header_from,
        "recipient": "agent@example.com",
        "subject": "Run the requested Arcwell task",
        "sanitizedText": "Please inspect STATUS.md and send a concise reply.",
        "auth": { "dmarc": "pass", "spf": "pass" },
        "warnings": []
    })
}
