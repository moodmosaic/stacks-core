#![no_main]

use blockstack_lib::net::api::callreadonly::RPCCallReadOnlyRequestHandler;
use blockstack_lib::net::http::common::HttpVersion;
use blockstack_lib::net::http::request::{HttpRequest, HttpRequestPreamble};
use blockstack_lib::net::http::HttpContentType;
use blockstack_lib::net::httpcore::decode_request_path;
use clarity::vm::costs::ExecutionCost;
use libfuzzer_sys::fuzz_target;
use serde_json::json;

// This fuzz target simulates HTTP requests to the call-read endpoint,
// exactly as they would come in through the REST API.
fuzz_target!(|data: &[u8]| {
    // Skip empty inputs.
    if data.is_empty() {
        return;
    }

    let sender_data = String::from_utf8_lossy(data).to_string();

    let json_body = json!({
        "sender": sender_data,
        "arguments": []
    });

    let stringified_body = json_body.to_string();
    let body = stringified_body.as_bytes();

    let url = "/v2/contracts/call-read/ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE/name/func";

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

    let (decoded_path, query) = decode_request_path(&preamble.path_and_query_str).unwrap();

    let execution_cost = ExecutionCost::max_value();

    let mut handler = RPCCallReadOnlyRequestHandler::new(1024 * 1024, execution_cost);

    let captures = handler.path_regex().captures(&decoded_path).unwrap();

    let _ = handler.try_parse_request(
        &preamble,
        &captures,
        if query.is_empty() { None } else { Some(&query) },
        body,
    );
});
