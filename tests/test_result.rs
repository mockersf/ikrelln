extern crate actix_web;
extern crate serde_json;
extern crate uuid;

extern crate ikrelln;

mod helpers;

use std::collections::HashMap;
use std::{thread, time};

use actix_web::*;

use ikrelln::api::span::IngestResponse;
use ikrelln::engine::test_result::TestResult;
use ikrelln::opentracing::span::Kind;
use ikrelln::opentracing::tags::IkrellnTags;
use ikrelln::opentracing::Span;

#[test]
fn should_create_test_result() {
    helpers::setup_logger();
    let mut srv = helpers::setup_server();

    let trace_id = uuid::Uuid::new_v4().to_string();

    let mut tags: HashMap<String, String> = HashMap::new();
    tags.insert(
        String::from({
            let tag: &str = IkrellnTags::Suite.into();
            tag
        }),
        "test_suite".to_string(),
    );
    tags.insert(
        String::from({
            let tag: &str = IkrellnTags::Class.into();
            tag
        }),
        "test_class".to_string(),
    );
    tags.insert(
        String::from({
            let tag: &str = IkrellnTags::Result.into();
            tag
        }),
        "success".to_string(),
    );

    let req = srv
        .client(http::Method::POST, "/api/v1/spans")
        .json(vec![Span {
            trace_id: trace_id.to_string(),
            id: trace_id.clone(),
            parent_id: None,
            name: Some("span_name".to_string()),
            kind: Some(Kind::CLIENT),
            duration: Some(25),
            timestamp: Some(50),
            debug: false,
            shared: false,
            local_endpoint: None,
            remote_endpoint: None,
            annotations: vec![],
            tags,
            binary_annotations: vec![],
        }])
        .unwrap();
    let response = srv.execute(req.send()).unwrap();
    assert!(response.status().is_success());
    let data: Result<IngestResponse, _> =
        serde_json::from_slice(&*srv.execute(response.body()).unwrap());
    assert!(data.is_ok());
    assert_eq!(data.unwrap().nb_events, 1);

    thread::sleep(time::Duration::from_millis(
        helpers::DELAY_RESULT_SAVED_MILLISECONDS,
    ));

    let req_tr = srv
        .client(
            http::Method::GET,
            &format!("/api/v1/testresults?traceId={}", &trace_id),
        )
        .finish()
        .unwrap();
    let response_tr = srv.execute(req_tr.send()).unwrap();
    assert!(response_tr.status().is_success());
    let data_tr: Result<Vec<TestResult>, _> =
        serde_json::from_slice(&*srv.execute(response_tr.body()).unwrap());
    assert!(data_tr.is_ok());
    println!("{:#?}", data_tr);
    assert_eq!(data_tr.unwrap().len(), 1);
    thread::sleep(time::Duration::from_millis(helpers::DELAY_FINISH));
}
