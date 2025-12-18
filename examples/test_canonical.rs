use orbit_observability::{EventPayload, OrbitEvent};

fn main() {
    let mut event = OrbitEvent::new(EventPayload::SpanStart {
        name: "with_retry_and_stats".to_string(),
        level: "INFO".to_string(),
    });

    event.trace_id = "335f8464197139ab59c4494274e55749".to_string();
    event.span_id = "4a63b017626d3de5".to_string();
    event.sequence = 0;
    event.integrity_hash = None;

    let canonical = serde_json::to_vec(&event).unwrap();
    let canonical_str = String::from_utf8(canonical.clone()).unwrap();

    println!("Canonical JSON from Rust:");
    println!("{}", canonical_str);
    println!();
    println!("Length: {}", canonical.len());
}
