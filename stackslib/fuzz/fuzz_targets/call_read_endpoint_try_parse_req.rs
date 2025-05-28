#![no_main]

use blockstack_lib::net::api::callreadonly::RPCCallReadOnlyRequestHandler;
use blockstack_lib::net::http::common::HttpVersion;
use blockstack_lib::net::http::request::{HttpRequest, HttpRequestPreamble};
use blockstack_lib::net::http::HttpContentType;
use blockstack_lib::net::httpcore::decode_request_path;
use clarity::vm::costs::ExecutionCost;
use libfuzzer_sys::fuzz_target;
use serde_json::json;

const MAX_MEMORY_LIMIT: u32 = 1024 * 1024; // 1MB
const TEST_PORT: u16 = 20443;
const TEST_HOST: &str = "localhost";
const CALL_READ_PATH: &str =
    "/v2/contracts/call-read/ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE/name/func";

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let input_string = String::from_utf8_lossy(data).to_string();
    let body_string = create_json_body(&input_string);
    let body = body_string.as_bytes();

    let preamble = create_test_preamble(body.len());

    let (decoded_path, query) = match decode_request_path(&preamble.path_and_query_str) {
        Ok(result) => result,
        Err(_) => return,
    };

    let mut handler =
        RPCCallReadOnlyRequestHandler::new(MAX_MEMORY_LIMIT, ExecutionCost::max_value());

    let captures = match handler.path_regex().captures(&decoded_path) {
        Some(captures) => captures,
        None => return,
    };

    let _result = handler.try_parse_request(
        &preamble,
        &captures,
        (!query.is_empty()).then_some(&query),
        body,
    );
});

fn create_json_body(input_string: &str) -> String {
    json!({
        "sender": input_string,
        "arguments": []
    })
    .to_string()
}

fn create_test_preamble(content_length: usize) -> HttpRequestPreamble {
    let mut preamble = HttpRequestPreamble::new(
        HttpVersion::Http11,
        "POST".to_string(),
        CALL_READ_PATH.to_string(),
        TEST_HOST.to_string(),
        TEST_PORT,
        false,
    );

    preamble.set_content_length(content_length as u32);
    preamble.content_type = Some(HttpContentType::JSON);
    preamble
}
