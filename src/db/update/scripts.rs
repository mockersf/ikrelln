use actix::prelude::*;
use chrono;
use diesel;
use diesel::prelude::*;

use crate::db::schema::script;
#[derive(Debug, Insertable, Queryable, Clone)]
#[table_name = "script"]
struct ScriptDb {
    id: String,
    name: String,
    source: String,
    script_type: i32,
    date_added: chrono::NaiveDateTime,
    status: i32,
}

#[derive(Message)]
pub struct SaveScript(pub crate::engine::streams::Script);

impl Handler<SaveScript> for super::DbExecutor {
    type Result = ();

    fn handle(&mut self, msg: SaveScript, _: &mut Self::Context) -> Self::Result {
        use super::super::schema::script::dsl::*;
        diesel::insert_into(script)
            .values(&ScriptDb {
                id: msg.0.id.expect("script should have an ID").clone(),
                name: msg.0.name.clone(),
                source: msg.0.source.clone(),
                script_type: msg.0.script_type.into(),
                date_added: msg.0.date_added.expect("script should have a date_added"),
                status: match msg.0.status.expect("script should have a status") {
                    crate::engine::streams::ScriptStatus::Enabled => 0,
                    crate::engine::streams::ScriptStatus::Disabled => 1,
                },
            })
            .execute(self.0.as_ref().expect("fail to get DB"))
            .unwrap();
    }
}

#[derive(Debug)]
pub struct DeleteScript(pub String);

impl Message for DeleteScript {
    type Result = Option<crate::engine::streams::Script>;
}

impl Handler<DeleteScript> for super::DbExecutor {
    type Result = MessageResult<DeleteScript>;

    fn handle(&mut self, msg: DeleteScript, _: &mut Self::Context) -> Self::Result {
        use super::super::schema::script::dsl::*;
        let script_found = script
            .filter(id.eq(&msg.0))
            .first::<ScriptDb>(self.0.as_ref().expect("fail to get DB"))
            .ok();

        diesel::delete(script.filter(id.eq(msg.0)))
            .execute(self.0.as_ref().expect("fail to get DB"))
            .ok();

        MessageResult(
            script_found.map(|script_from_db| crate::engine::streams::Script {
                id: Some(script_from_db.id.clone()),
                date_added: Some(script_from_db.date_added),
                script_type: script_from_db.script_type.into(),
                name: script_from_db.name.clone(),
                source: script_from_db.source.clone(),
                status: Some(match script_from_db.status {
                    0 => crate::engine::streams::ScriptStatus::Enabled,
                    _ => crate::engine::streams::ScriptStatus::Disabled,
                }),
            }),
        )
    }
}

#[derive(Message)]
pub struct UpdateScript(pub crate::engine::streams::Script);

impl Handler<UpdateScript> for super::DbExecutor {
    type Result = ();

    fn handle(&mut self, msg: UpdateScript, _: &mut Self::Context) -> Self::Result {
        use super::super::schema::script::dsl::*;
        diesel::update(script.filter(id.eq(&msg.0.id.expect("script should have an ID"))))
            .set((
                name.eq(&msg.0.name),
                source.eq(&msg.0.source),
                status.eq(match msg.0.status.expect("script should have a status") {
                    crate::engine::streams::ScriptStatus::Enabled => 0,
                    crate::engine::streams::ScriptStatus::Disabled => 1,
                }),
            ))
            .execute(self.0.as_ref().expect("fail to get DB"))
            .unwrap();
    }
}
