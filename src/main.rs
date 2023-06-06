
use fastly::http::{header, Method, StatusCode};
use fastly::{Error, Request, Response, KVStore};
use fastly_cache_preview::{insert, CacheKey, Transaction};
use serde_json::json;
use std::io::Write;
use std::time::{Duration, Instant};

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
        "/" => Request::get("https://jessehill.github.io").send("jessehill.github.io")?,

        "/lookup-test" => run_lookup_test(req),

        "/kv-lookup-test" => run_kv_lookup_test(req),

        // Catch all other requests and return a 404.
        _ => Response::from_status(StatusCode::NOT_FOUND)
            .with_body_text_plain("The page you requested could not be found\n"),
    };
    Ok(response.with_header("X-Service-Name", "JesseCacheDService"))
}

fn extract_params(req: &Request) -> (i32, String) {
    let iterations_str = req.get_query_parameter("iterations").unwrap();
    let iterations = iterations_str.parse::<i32>().unwrap();

    let key = req.get_query_parameter("key").unwrap_or("some sort of cache key").to_string();

    (iterations, key)
}

fn insert_cache_object(key: &CacheKey, message: &str) {
    match Transaction::lookup(key.clone()).execute() {
        Ok(handle) => {
            let found = handle.found();
            if !found.is_some() {
                let mut writer = insert(key.clone(), Duration::from_secs(600))
                    .execute()
                    .unwrap();
                writer.write_all(message.as_bytes()).unwrap();
                writer.finish().unwrap();
            }
        }
        Err(_) => {
            println!("Error looking up cache key.");
        },
    }
}

fn run_lookup_test(req: Request) -> Response {
    let (iterations, key) = extract_params(&req);

    println!("--- Running low-level lookup test: 1 ---");

    let key = CacheKey::from(key);
    insert_cache_object(&key, "Hello! Let's test some latency!");

    let mut results = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        let lookup_handle = Transaction::lookup(key.clone()).execute().unwrap();
        let found = lookup_handle.found().unwrap();
        let body = found.to_stream().unwrap().into_string();
        results.push(json!({
            "body": body,
            "lookupTimeTaken": start.elapsed(),
            "found": true
        }));
    }

    Response::from_status(StatusCode::OK)
        .with_body_json(&results)
        .unwrap()
}

fn run_kv_lookup_test(req: Request) -> Response {
    let (iterations, key) = extract_params(&req);
    
    println!("--- Running kv lookup test: 1 ---");

    let mut store = KVStore::open("jesse_kv_store").unwrap().unwrap();
    store.insert(&key, "Hello! Let's test some latency!").unwrap();
    // Fetch once before the timed loop to warm things up.
    store.lookup(&key).unwrap().unwrap();

    let mut results = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        let body = store.lookup(&key).unwrap().unwrap();
        results.push(json!({
            "body": body.into_string(),
            "lookupTimeTaken": start.elapsed(),
            "found": true
        }));
    }

    Response::from_status(StatusCode::OK)
        .with_body_json(&results)
        .unwrap()
}
