use fastly::http::{header, Method, StatusCode};
use fastly::kv_store::KVStore;
use fastly::{Error, Request, Response};
// use fastly_cache_preview::{CacheKey, Transaction, insert};
use fastly_cache_preview::handle::{
    lookup, transaction_lookup, CacheKey, CacheLookupState, GetBodyOptions, LookupOptions,
    WriteOptions,
};

use serde_json::json;
use std::time::Instant;

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

        // "/lookup-test" => run_low_level_lookup_test(req),
        "/lookup-test-old-school" => run_low_level_test_old_school(req),

        "/kv-lookup-test" => run_kv_lookup_test(req),

        // Catch all other requests and return a 404.
        _ => Response::from_status(StatusCode::NOT_FOUND)
            .with_body_text_plain("The page you requested could not be found\n"),
    };
    Ok(response.with_header("X-Service-Name", "JesseCacheDService"))
}

// fn insert_dummy_object(key: CacheKey) {
//     let message = "Hello this is dog";
//     let mut writer = insert(key.clone(), Duration::from_secs(600)).execute().unwrap();
//     writer.write_all(message.as_bytes()).unwrap();
//     writer.finish().unwrap();
// }

// fn run_low_level_lookup_test(req: Request) -> Response {
//     println!("Running lookup test");

//     let key = key_for_req(&req);
//     println!("Inserting dummy object");
//     insert_dummy_object(key.clone());

//     let iterations_str = req.get_query_parameter("iterations").unwrap();
//     let iterations = iterations_str.parse::<i32>().unwrap();

//     let mut results = Vec::new();
//     println!("Looking up object");
//     for _ in 0..iterations {
//         let start = Instant::now();
//         let lookup_handle = Transaction::lookup(key.clone()).execute().unwrap();
//         let found = lookup_handle.found().unwrap();
//         let body = found.to_stream().unwrap().into_string();
//         // println!("Got result for iteration {}", num);
//         results.push(json!({
//             // "key": key,
//             // "body": body,
//             "lookupTimeTaken": start.elapsed(),
//             "found": true
//         }));
//     }

//     println!("Finished looking up object");
//     Response::from_status(StatusCode::OK).with_body_json(&results).unwrap()
// }

fn run_low_level_test_old_school(req: Request) -> Response {
    println!(" --- Running lookup test 4 --- ");
    let key: CacheKey = CacheKey::from("a random key");
    let lookup_handle = transaction_lookup(key.clone(), &LookupOptions::default()).unwrap();
    let mut body_handle = lookup_handle.transaction_insert(&WriteOptions::default()).unwrap();
    body_handle.write_str("Hello this is dog");
    body_handle.finish().unwrap();

    match lookup(key.clone(), &LookupOptions::default()) {
        Ok(cache_handle) => {
            let lookup_state = cache_handle.get_state().unwrap();
            println!("cache_handle found: {}", lookup_state.contains(CacheLookupState::FOUND));
            println!("cache_handle stale: {}", lookup_state.contains(CacheLookupState::STALE));
            println!("cache_handle usable: {}", lookup_state.contains(CacheLookupState::USABLE));
            println!("cache_handle must_insert?: {}", lookup_state.contains(CacheLookupState::MUST_INSERT_OR_UPDATE));
            match cache_handle.get_body(&GetBodyOptions::default()) {
            Ok(body_handle) => {
                if body_handle.is_some() {
                    let body = body_handle.unwrap().into_string();
                    println!("Found body from lookup: {}", body);
                } else {
                    println!("Body handle was empty");
                }
            }
            Err(message) => {
                println!("Error on get_body: {}", message.code);
            }
        }
        },
        Err(message) => {
            println!("Error on lookup: {}", message.code);
        }
    }

    let iterations_str = req.get_query_parameter("iterations").unwrap();
    let iterations = iterations_str.parse::<i32>().unwrap();

    let mut results = Vec::new();
    for _ in 0..iterations {
        let start = Instant::now();
        match transaction_lookup(key.clone(), &LookupOptions::default()) {
            Ok(lookup_handle) => {
                println!(
                    "Usable: {}",
                    lookup_handle
                        .get_state()
                        .unwrap()
                        .contains(CacheLookupState::USABLE)
                );
                println!(
                    "Found: {}",
                    lookup_handle
                        .get_state()
                        .unwrap()
                        .contains(CacheLookupState::FOUND)
                );
                println!(
                    "Stale: {}",
                    lookup_handle
                        .get_state()
                        .unwrap()
                        .contains(CacheLookupState::STALE)
                );
                println!(
                    "Must insert: {}",
                    lookup_handle
                        .get_state()
                        .unwrap()
                        .contains(CacheLookupState::MUST_INSERT_OR_UPDATE)
                );
                match lookup_handle.get_state() {
                    Ok(lookup_handle_state) => {
                        if lookup_handle_state.contains(CacheLookupState::USABLE) {
                            let body_handle = lookup_handle
                                .get_body(&GetBodyOptions::default())
                                .unwrap()
                                .unwrap();
                            let body = body_handle.into_string();
                            println!("Found hit");
                            results.push(json!({
                                "body": body,
                                "lookupTimeTaken": start.elapsed(),
                                "found": true
                            }));
                        } else {
                            if lookup_handle_state.contains(CacheLookupState::MUST_INSERT_OR_UPDATE)
                            {
                                println!("Got must insert object");
                            }
                            if lookup_handle_state.contains(CacheLookupState::FOUND) {
                                println!("Got found object");
                            }
                            if lookup_handle_state.contains(CacheLookupState::STALE) {
                                println!("Got stale object");
                            }
                        }
                        if !lookup_handle_state.contains(CacheLookupState::USABLE)
                            && !lookup_handle_state.contains(CacheLookupState::FOUND)
                            && !lookup_handle_state.contains(CacheLookupState::STALE)
                            && !lookup_handle_state
                                .contains(CacheLookupState::MUST_INSERT_OR_UPDATE)
                        {
                            println!("No flag set on the lookup state");
                        }
                    }
                    Err(_) => {
                        println!("Failed to get lookup handle state")
                    }
                }
            }
            Err(_) => {
                println!("Failed to lookup object")
            }
        }
    }

    println!("Finished looking up object");
    Response::from_status(StatusCode::OK)
        .with_body_json(&results)
        .unwrap()

    // let r = match transaction_lookup(cache_key, &LookupOptions::default()) {
    //     Ok(lookup_handle) => {
    //         st.add("transaction_lookup ok");
    //         match lookup_handle.get_state() {
    //             Ok(lookup_handle_status) => {
    //                 if lookup_handle_status.contains(CacheLookupState::USABLE) {
    //                     cache_status = "HIT";
    //                     st.add("lookup_handle_status status USABLE");
    //                     // We have the data just use it
    //                     self.handle_usable(lookup_handle, CacheStatus::Hit, st)
    //                 } else if lookup_handle_status.contains(
    //                     CacheLookupState::MUST_INSERT_OR_UPDATE
    //                         | CacheLookupState::FOUND
    //                         | CacheLookupState::STALE,
    //                 ) {
    //                     // Revalidate using the metadata
    //                     st.add("lookup_handle_status status revalidate");
    //                     self.handle_refetch(req, lookup_handle, true, st)
    //                 } else {
    //                     // fetch
    //                     st.add("lookup_handle_status status fetch");
    //                     self.handle_refetch(req, lookup_handle, false, st)
    //                 }
    //             }
    //             Err(_) => Err(anyhow!("failed to get lookup status")),
    //         }
    //     }
    //     Err(_) => Err(anyhow!("failed to get transaction_lookup")),
    // };
}

// fn key_for_req(req: &Request) -> CacheKey {
//     let mut hasher = Sha256::new();
//     hasher.update(req.get_path().as_bytes());
//     hasher.update(get_pop().as_bytes());
//     Bytes::copy_from_slice(&hasher.finalize())
// }

fn run_kv_lookup_test(req: Request) -> Response {
    let mut store = KVStore::open("jesse_kv_store").unwrap().unwrap();
    let key = "firstKey";
    store.insert(&key, "Yes, hello! This is dog.").unwrap();

    let iterations_str = req.get_query_parameter("iterations").unwrap();
    let iterations = iterations_str.parse::<i32>().unwrap();

    let mut results = Vec::new();
    println!("Looking up kv object");
    for _ in 0..iterations {
        let start = Instant::now();
        let body = store.lookup(key).unwrap().unwrap();
        // println!("Got result for iteration {}", num);
        results.push(json!({
            // "key": key,
            "body": body.into_string(),
            "lookupTimeTaken": start.elapsed(),
            "found": true
        }));
    }

    println!("Finished looking up kv object");
    Response::from_status(StatusCode::OK)
        .with_body_json(&results)
        .unwrap()
}

fn get_pop() -> String {
    std::env::var("FASTLY_POP").unwrap()
}
