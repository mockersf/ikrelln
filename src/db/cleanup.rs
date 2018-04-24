use diesel;
use actix::{Handler, Message};
use diesel::prelude::*;
use chrono;
use std::time::Duration;
use actix::prelude::*;

pub struct CleanUp;
impl Message for CleanUp {
    type Result = ();
}
impl Handler<CleanUp> for super::DbExecutor {
    type Result = ();

    fn handle(&mut self, _msg: CleanUp, _ctx: &mut Self::Context) -> Self::Result {
        use super::schema::test_result::dsl::*;

        let deleted = {
            let limit = chrono::Utc::now().naive_utc()
                - chrono::Duration::milliseconds(::CONFIG.cleanup.delay_test_results as i64);

            test_result
                .filter(date.lt(limit))
                .filter(cleanup_status.eq(super::test::ResultCleanupStatus::Shell.into_i32()))
                .load::<super::test::TestResultDb>(self.0.as_ref().unwrap())
                .ok()
                .unwrap_or_else(|| vec![])
                .iter()
                .for_each(|tr| {
                    use super::schema::test_result_in_report::dsl::*;

                    diesel::delete(
                        test_result_in_report
                            .filter(trace_id.eq(&tr.trace_id).and(test_id.eq(&tr.test_id))),
                    ).execute(self.0.as_ref().unwrap())
                        .ok();
                });

            diesel::delete(
                test_result
                    .filter(date.lt(limit))
                    .filter(cleanup_status.eq(super::test::ResultCleanupStatus::Shell.into_i32())),
            ).execute(self.0.as_ref().unwrap())
                .unwrap()
        };

        let to_clean: Vec<super::test::TestResultDb> = {
            let limit = chrono::Utc::now().naive_utc()
                - chrono::Duration::milliseconds(::CONFIG.cleanup.delay_spans as i64);

            let to_clean = test_result
                .filter(date.lt(limit))
                .filter(cleanup_status.eq(super::test::ResultCleanupStatus::WithData.into_i32()))
                .load::<super::test::TestResultDb>(self.0.as_ref().unwrap())
                .ok()
                .unwrap_or_else(|| vec![]);

            diesel::update(
                test_result.filter(date.lt(limit)).filter(
                    cleanup_status.eq(super::test::ResultCleanupStatus::WithData.into_i32()),
                ),
            ).set(cleanup_status.eq(super::test::ResultCleanupStatus::Shell.into_i32()))
                .execute(self.0.as_ref().unwrap())
                .ok();

            to_clean
        };

        to_clean.iter().for_each(|tr| {
            use super::schema::span::dsl::*;

            let spans: Vec<super::span::SpanDb> = {
                span.filter(trace_id.eq(&tr.trace_id))
                    .load::<super::span::SpanDb>(self.0.as_ref().unwrap())
                    .ok()
                    .unwrap_or_else(|| vec![])
            };

            spans.iter().for_each(|spandb| {
                {
                    use super::schema::annotation::dsl::*;

                    diesel::delete(
                        annotation
                            .filter(trace_id.eq(&spandb.trace_id).and(span_id.eq(&spandb.id))),
                    ).execute(self.0.as_ref().unwrap())
                        .ok();
                }

                {
                    use super::schema::tag::dsl::*;

                    diesel::delete(tag.filter(span_id.eq(&spandb.id)))
                        .execute(self.0.as_ref().unwrap())
                        .ok();
                }
            });

            diesel::delete(span.filter(trace_id.eq(&tr.trace_id)))
                .execute(self.0.as_ref().unwrap())
                .ok();
        });

        let reports_cleaned = {
            let limit = chrono::Utc::now().naive_utc()
                - chrono::Duration::milliseconds(::CONFIG.cleanup.delay_reports as i64);

            use super::schema::report::dsl::*;

            report
                .filter(last_update.lt(limit))
                .load::<super::reports::ReportDb>(self.0.as_ref().unwrap())
                .ok()
                .unwrap_or_else(|| vec![])
                .iter()
                .for_each(|rep| {
                    use super::schema::test_result_in_report::dsl::*;

                    diesel::delete(test_result_in_report.filter(report_id.eq(&rep.id)))
                        .execute(self.0.as_ref().unwrap())
                        .ok();
                });

            diesel::delete(report.filter(last_update.lt(limit)))
                .execute(self.0.as_ref().unwrap())
                .unwrap()
        };

        info!(
            "deleted {} test results, cleaned {} and removed {} reports",
            deleted,
            to_clean.len(),
            reports_cleaned,
        );
    }
}

pub struct CleanUpTimer;
impl Actor for CleanUpTimer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        ctx.notify_later(
            Trigger,
            Duration::from_millis(::CONFIG.cleanup.schedule as u64),
        );
    }
}

#[derive(Message)]
struct Trigger;
impl Handler<Trigger> for CleanUpTimer {
    type Result = ();
    fn handle(&mut self, _msg: Trigger, ctx: &mut Self::Context) -> Self::Result {
        ::DB_EXECUTOR_POOL.do_send(CleanUp);
        ctx.notify_later(
            Trigger,
            Duration::from_millis(::CONFIG.cleanup.schedule as u64),
        );
    }
}