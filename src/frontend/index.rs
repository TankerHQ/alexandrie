use bigdecimal::{BigDecimal, ToPrimitive};
use diesel::prelude::*;
use json::json;
use tide::{Context, Response};

use crate::config::State;
use crate::db::schema::*;
use crate::error::Error;
use crate::frontend::helpers;
use crate::utils;

pub(crate) async fn route(ctx: Context<State>) -> Result<Response, Error> {
    let state = ctx.state();
    let repo = &state.repo;

    let crate_count = repo
        .run(|conn| {
            crates::table
                .select(diesel::dsl::count(crates::id))
                .first::<i64>(&conn)
        })
        .await?;
    let total_downloads = repo
        .run(|conn| {
            crates::table
                .select(diesel::dsl::sum(crates::downloads))
                .first::<Option<BigDecimal>>(&conn)
        })
        .await?
        .map_or(0, |dec| {
            dec.to_u64()
                .expect("download count exceeding u64::max_value()")
        });
    let most_downloaded = repo
        .run(|conn| {
            crates::table
                .select((crates::name, crates::downloads))
                .order_by(crates::downloads.desc())
                .limit(10)
                .load::<(String, u64)>(&conn)
        })
        .await?;
    let last_updated = repo
        .run(|conn| {
            crates::table
                .select((crates::name, crates::updated_at))
                .order_by(crates::updated_at.desc())
                .limit(10)
                .load::<(String, chrono::NaiveDateTime)>(&conn)
        })
        .await?;

    let engine = &state.frontend.handlebars;
    let context = json!({
        "instance": &state.frontend.config,
        "total_downloads": helpers::humanize_number(total_downloads),
        "crate_count": helpers::humanize_number(crate_count),
        "most_downloaded": most_downloaded.into_iter().map(|(name, downloads)| json!({
            "name": name,
            "downloads": helpers::humanize_number(downloads),
        })).collect::<Vec<_>>(),
        "last_updated": last_updated.into_iter().map(|(name, date)| json!({
            "name": name,
            "updated_at": helpers::humanize_datetime(date),
        })).collect::<Vec<_>>(),
    });
    Ok(utils::response::html(
        engine.render("index", &context).unwrap(),
    ))
}
