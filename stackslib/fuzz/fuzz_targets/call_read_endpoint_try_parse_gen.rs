#![no_main]

use blockstack_lib::net::api::callreadonly::RPCCallReadOnlyRequestHandler;
use blockstack_lib::net::http::common::HttpVersion;
use blockstack_lib::net::http::request::{HttpRequest, HttpRequestPreamble};
use blockstack_lib::net::http::HttpContentType;
use blockstack_lib::net::httpcore::decode_request_path;
use clarity::vm::costs::ExecutionCost;
use libfuzzer_sys::fuzz_target;
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    // Use the input data as both the sender and the path. This makes the fuzz
    // target a raw one, allowing any input to be thrown at `call-read`'s
    // `try_parse_request` method.
    let sender_data = String::from_utf8_lossy(data).to_string();
    let path = String::from_utf8_lossy(data).to_string();

    let json_body = json!({
        "sender": sender_data,
        "arguments": []
    });

    let stringified_body = json_body.to_string();
    let body = stringified_body.as_bytes();

    let url = format!("/v2/contracts/call-read/{}", path);

    let mut preamble = HttpRequestPreamble::new(
        HttpVersion::Http11,
        "POST".to_string(),
        url.to_string(),
        "localhost".to_string(),
        20443,
        false,
    );
    preamble.set_content_length(body.len() as u32);
    preamble.content_type = Some(HttpContentType::JSON);

    if let Ok((decoded_path, query)) = decode_request_path(&preamble.path_and_query_str) {
        let execution_cost = ExecutionCost::max_value();
        let mut handler = RPCCallReadOnlyRequestHandler::new(1024 * 1024, execution_cost);

        if let Some(captures) = handler.path_regex().captures(&decoded_path) {
            let _ = handler.try_parse_request(
                &preamble,
                &captures,
                if query.is_empty() { None } else { Some(&query) },
                body,
            );
        } else {
            return;
        }
    } else {
        return;
    }
});
