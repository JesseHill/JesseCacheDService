use fastly::http::{header, Method, StatusCode};
use fastly::{Error, Request, Response};
use fastly_cache_preview::{CacheKey, Transaction, insert};
use std::io::Write;
use std::time::{Duration, Instant};
use sha2::{Digest, Sha256};
use bytes::Bytes;
use serde_json::json;

#[fastly::main]
fn main(req: Request) -> Result<Response, Error> {
    match req.get_method() {
        // Allow GET and HEAD requests.
        &Method::GET | &Method::HEAD => (),

        // Deny anything else.
        _ => {
            return Ok(Response::from_status(StatusCode::METHOD_NOT_ALLOWED)
                .with_header(header::ALLOW, "GET, HEAD")
                .with_body_text_plain("This method is not allowed\n"))
        }
    };

    let response = match req.get_path() {
        "/" => {
            Request::get("https://jessehill.github.io")
                .send("jessehill.github.io")?
        }

        "/lookup-test" => {
            run_lookup_test(req)
        }

        // Catch all other requests and return a 404.
        _ => Response::from_status(StatusCode::NOT_FOUND)
            .with_body_text_plain("The page you requested could not be found\n"),
    };
    Ok(response.with_header("X-Service-Name", "JesseCacheDService"))
}


fn insert_dummy_object(key: CacheKey) {
    let message = "Hello this is dog";
    let mut writer = insert(key.clone(), Duration::from_secs(600)).execute().unwrap();
    writer.write_all(message.as_bytes()).unwrap();
    writer.finish().unwrap();
}

fn run_lookup_test(req: Request) -> Response {
    println!("Running lookup test");
    
    let key = key_for_req(&req);
    println!("Inserting dummy object");
    insert_dummy_object(key.clone());

    let amount_str = req.get_query_parameter("amount").unwrap();
    let iterations = amount_str.parse::<i32>().unwrap();

    let mut results = Vec::new();
    println!("Looking up object");
    for _ in 0..iterations {
        let start = Instant::now();
        let lookup_handle = Transaction::lookup(key.clone()).execute().unwrap();
        let found = lookup_handle.found().unwrap();
        let body = found.to_stream().unwrap().into_string();
        // println!("Got result for iteration {}", num);
        results.push(json!({
            // "key": key,
            // "body": body,
            "lookupTimeTaken": start.elapsed(),
            "found": true
        }));
    }

    println!("Finished looking up object");
    Response::from_status(StatusCode::OK).with_body_json(&results).unwrap()
}

fn key_for_req(req: &Request) -> CacheKey {
    let mut hasher = Sha256::new();
    hasher.update(req.get_path().as_bytes());
    hasher.update(get_pop().as_bytes());
    Bytes::copy_from_slice(&hasher.finalize())
}

fn get_pop() -> String {
    std::env::var("FASTLY_POP").unwrap()
}