use serde_json::{json, Value};

pub fn execute(params: &Value) -> Value {
    let result_id = params
        .get("result_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let action = params
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("enter");

    json!({
        "ok": true,
        "executed": !result_id.is_empty(),
        "result_id": result_id,
        "action": action,
    })
}
