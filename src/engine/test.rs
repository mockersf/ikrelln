use std::str::FromStr;
use std::collections::HashMap;
use std::time::Duration;

use futures::{self, Future};
use actix::prelude::*;

#[derive(Debug)]
struct KnownTag {
    tag: String,
}
impl From<OpenTracingTag> for KnownTag {
    fn from(tag: OpenTracingTag) -> KnownTag {
        let tag_str: &'static str = tag.into();
        KnownTag {
            tag: format!("{}", tag_str),
        }
    }
}
impl From<IkrellnTags> for KnownTag {
    fn from(tag: IkrellnTags) -> KnownTag {
        let tag_str: &'static str = tag.into();
        KnownTag {
            tag: format!("{}", tag_str),
        }
    }
}

// OpenTracing semantics v1.1
// https://github.com/opentracing/specification/blob/master/semantic_conventions.md#span-tags-table
#[derive(Clone)]
enum OpenTracingTag {
    Component,
    DbInstance,
    DbStatement,
    DbType,
    DbUser,
    Error,
    HttpMethod,
    HttpStatusCode,
    HttpUrl,
    MessageBusDestination,
    PeerAddress,
    PeerHostname,
    PeerIpv4,
    PeerIpv6,
    PeerPort,
    PeerService,
    SamplingPriority,
    SpanKind,
}
impl From<OpenTracingTag> for &'static str {
    fn from(tag: OpenTracingTag) -> &'static str {
        match tag {
            OpenTracingTag::Component => "component",
            OpenTracingTag::DbInstance => "db.instance",
            OpenTracingTag::DbStatement => "db.statement",
            OpenTracingTag::DbType => "db.type",
            OpenTracingTag::DbUser => "db.user",
            OpenTracingTag::Error => "error",
            OpenTracingTag::HttpMethod => "http.method",
            OpenTracingTag::HttpStatusCode => "http.status_code",
            OpenTracingTag::HttpUrl => "http.url",
            OpenTracingTag::MessageBusDestination => "message_bus.destination",
            OpenTracingTag::PeerAddress => "peer.address",
            OpenTracingTag::PeerHostname => "peer.hostname",
            OpenTracingTag::PeerIpv4 => "peer.ipv4",
            OpenTracingTag::PeerIpv6 => "peer.ipv6",
            OpenTracingTag::PeerPort => "peer.port",
            OpenTracingTag::PeerService => "peer.service",
            OpenTracingTag::SamplingPriority => "sampling.priority",
            OpenTracingTag::SpanKind => "span.kind",
        }
    }
}
struct NonOpenTracingTag;
impl FromStr for OpenTracingTag {
    type Err = NonOpenTracingTag;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "component" => Ok(OpenTracingTag::Component),
            "db.instance" => Ok(OpenTracingTag::DbInstance),
            "db.statement" => Ok(OpenTracingTag::DbStatement),
            "db.type" => Ok(OpenTracingTag::DbType),
            "db.user" => Ok(OpenTracingTag::DbUser),
            "error" => Ok(OpenTracingTag::Error),
            "http.method" => Ok(OpenTracingTag::HttpMethod),
            "http.status_code" => Ok(OpenTracingTag::HttpStatusCode),
            "http.url" => Ok(OpenTracingTag::HttpUrl),
            "message_bus.destination" => Ok(OpenTracingTag::MessageBusDestination),
            "peer.address" => Ok(OpenTracingTag::PeerAddress),
            "peer.hostname" => Ok(OpenTracingTag::PeerHostname),
            "peer.ipv4" => Ok(OpenTracingTag::PeerIpv4),
            "peer.ipv6" => Ok(OpenTracingTag::PeerIpv6),
            "peer.port" => Ok(OpenTracingTag::PeerPort),
            "peer.service" => Ok(OpenTracingTag::PeerService),
            "sampling.priority" => Ok(OpenTracingTag::SamplingPriority),
            "span.kind" => Ok(OpenTracingTag::SpanKind),
            &_ => Err(NonOpenTracingTag),
        }
    }
}

#[derive(Clone)]
enum IkrellnTags {
    TestClass,
    TestEnvironment,
    TestName,
    TestResult,
    TestStepParameter,
    TestStepStatus,
    TestStepType,
    TestSuite,
}
impl From<IkrellnTags> for &'static str {
    fn from(tag: IkrellnTags) -> &'static str {
        match tag {
            IkrellnTags::TestClass => "test.class",
            IkrellnTags::TestEnvironment => "test.environment",
            IkrellnTags::TestName => "test.name",
            IkrellnTags::TestResult => "test.result",
            IkrellnTags::TestStepParameter => "test.step_parameter",
            IkrellnTags::TestStepStatus => "test.step_status",
            IkrellnTags::TestStepType => "test.step_type",
            IkrellnTags::TestSuite => "test.suite",
        }
    }
}
struct NonIkrellnTag;
impl FromStr for IkrellnTags {
    type Err = NonIkrellnTag;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "test.class" => Ok(IkrellnTags::TestClass),
            "test.environment" => Ok(IkrellnTags::TestEnvironment),
            "test.name" => Ok(IkrellnTags::TestName),
            "test.result" => Ok(IkrellnTags::TestResult),
            "test.step_parameter" => Ok(IkrellnTags::TestStepParameter),
            "test.step_status" => Ok(IkrellnTags::TestStepStatus),
            "test.step_type" => Ok(IkrellnTags::TestStepType),
            "test.suite" => Ok(IkrellnTags::TestSuite),
            &_ => Err(NonIkrellnTag),
        }
    }
}

#[derive(Default)]
pub struct TraceParser;
impl Actor for TraceParser {
    type Context = Context<Self>;
}
impl actix::Supervised for TraceParser {}

impl actix::SystemService for TraceParser {
    fn service_started(&mut self, _ctx: &mut Context<Self>) {}
}

#[derive(Message)]
pub struct TraceDoneNow(pub String);
impl Handler<TraceDoneNow> for TraceParser {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: TraceDoneNow, ctx: &mut Context<Self>) -> Self::Result {
        ctx.notify_later(TraceDone(msg.0), Duration::new(10, 0));
        Ok(())
    }
}

#[derive(Message)]
pub struct TraceDone(pub String);
impl Handler<TraceDone> for TraceParser {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: TraceDone, ctx: &mut Context<Self>) -> Self::Result {
        let trace_parser = ::DB_EXECUTOR_POOL
            .call_fut(::db::span::GetSpans(
                ::db::span::SpanQuery::default()
                    .with_trace_id(msg.0)
                    .with_limit(1000),
            ))
            .from_err()
            .and_then(|spans| {
                if let Ok(spans) = spans {
                    let mut _spans_processed: Vec<String> = vec![];
                    let main_span = spans.iter().find(|span| span.parent_id.is_none()).unwrap();
                    let te = TestExecution::try_from(main_span);
                    match te {
                        Ok(te) => Ok(Some(te)),
                        Err(tag) => {
                            warn!(
                                "missing / invalid tag {:?} in trace {:?} main span",
                                tag,
                                main_span.trace_id
                            );
                            Ok(None)
                        }
                    }
                } else {
                    Ok(None)
                }
            });
        ctx.add_future(trace_parser.and_then(|test_exec| match test_exec {
            Some(test_exec) => futures::future::result(Ok(TestExecutionToSave(test_exec))),
            None => futures::future::result(Err(futures::Canceled)),
        }));

        Ok(())
    }
}

#[derive(Message, Debug)]
pub struct TestExecutionToSave(TestExecution);
impl Handler<Result<TestExecutionToSave, futures::Canceled>> for TraceParser {
    type Result = Result<(), ()>;
    fn handle(
        &mut self,
        msg: Result<TestExecutionToSave, futures::Canceled>,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        if let Ok(test_execution) = msg {
            info!("got a test execution parsed: {:?}", test_execution);
        }

        Ok(())
    }
}

#[derive(Debug)]
enum TestResult {
    Success,
    Failure,
    Skipped,
}
impl TestResult {
    fn try_from(s: String) -> Result<Self, KnownTag> {
        match s.to_lowercase().as_ref() {
            "success" => Ok(TestResult::Success),
            "failure" => Ok(TestResult::Failure),
            "skipped" => Ok(TestResult::Skipped),
            _ => Err(IkrellnTags::TestResult.into()),
        }
    }
}

#[derive(Debug)]
struct TestExecution {
    suite: String,
    class: String,
    name: String,
    trace_id: String,
    date: i64,
    result: TestResult,
    duration: i64,
    environment: Option<String>,
}

impl TestExecution {
    fn value_from_tag<T>(tags: &HashMap<String, String>, tag: T) -> Result<String, KnownTag>
    where
        T: Clone,
        KnownTag: From<T>,
        &'static str: From<T>,
    {
        tags.get(tag.clone().into())
            .ok_or(tag.into())
            .map(|v| v.to_string())
    }
    fn value_from_tag_or(
        span: &::engine::span::Span,
        tag: IkrellnTags,
        f: fn(&::engine::span::Span) -> Option<String>,
    ) -> Result<String, KnownTag> {
        match span.tags
            .get(tag.clone().into())
            .ok_or(tag.into())
            .map(|v| v.to_string())
        {
            Ok(value) => Ok(value),
            Err(err) => f(span).ok_or(err),
        }
    }

    fn try_from(span: &::engine::span::Span) -> Result<Self, KnownTag> {
        Ok(TestExecution {
            suite: Self::value_from_tag_or(&span, IkrellnTags::TestSuite, |span| {
                span.local_endpoint.clone().and_then(|ep| ep.service_name)
            })?,
            class: Self::value_from_tag(&span.tags, IkrellnTags::TestClass)?,
            name: Self::value_from_tag_or(&span, IkrellnTags::TestName, |span| span.name.clone())?,
            trace_id: span.trace_id.clone(),
            date: span.timestamp.ok_or(KnownTag {
                tag: "ts".to_string(),
            })?,
            result: TestResult::try_from(Self::value_from_tag_or(
                &span,
                IkrellnTags::TestResult,
                |span| {
                    Self::value_from_tag(&span.tags, OpenTracingTag::Error)
                        .ok()
                        .map(|v| match v.to_lowercase().as_ref() {
                            "true" => "failure".to_string(),
                            other => other.to_string(),
                        })
                },
            )?)?,
            duration: span.duration.ok_or(KnownTag {
                tag: "duration".to_string(),
            })?,
            environment: Self::value_from_tag(&span.tags, IkrellnTags::TestEnvironment).ok(),
        })
    }
}