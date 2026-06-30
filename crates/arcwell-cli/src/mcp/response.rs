use super::*;

pub(crate) fn mcp_structured_content(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        other => json!({ "result": other }),
    }
}

pub(crate) fn mcp_tool_response_value(name: &str, value: Value) -> Value {
    if name == "ops_snapshot" {
        mcp_compact_ops_snapshot(value)
    } else {
        value
    }
}

pub(crate) fn mcp_compact_ops_snapshot(value: Value) -> Value {
    let Some(object) = value.as_object() else {
        return value;
    };
    let counts = object
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_array()
                .map(|items| (key.clone(), json!(items.len())))
        })
        .collect::<serde_json::Map<_, _>>();

    json!({
        "summary": "Compact MCP ops snapshot. Use `arcwell ops` for the full local JSON payload.",
        "health": object.get("health").cloned().unwrap_or_else(|| json!({})),
        "backlog": object.get("backlog").cloned().unwrap_or_else(|| json!({})),
        "secret_health": object.get("secret_health").cloned().unwrap_or_else(|| json!({})),
        "x_stats": object.get("x_stats").cloned().unwrap_or_else(|| json!({})),
        "counts": counts
    })
}
