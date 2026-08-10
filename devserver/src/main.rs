//! Browser dev server: exposes the same commands as the Tauri app over HTTP
//! so the frontend can be developed and tested in a plain browser.
//! POST /rpc/<command> with a JSON body identical to the Tauri invoke args.

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use paratwin_engine as engine;
use paratwin_host::Host;
use serde_json::{json, Value};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

type App = Arc<Host>;

fn err(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

async fn rpc(
    State(host): State<App>,
    axum::extract::Path(cmd): axum::extract::Path<String>,
    Json(args): Json<Value>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let host2 = host.clone();
    let cmd2 = cmd.clone();
    // simulations are CPU-heavy: run on a blocking thread
    let out = tokio::task::spawn_blocking(move || dispatch(&host2, &cmd2, args))
        .await
        .map_err(err)?
        .map_err(err)?;
    Ok(Json(out))
}

fn dispatch(host: &Host, cmd: &str, args: Value) -> anyhow::Result<Value> {
    let v = match cmd {
        "list_scenarios" => serde_json::to_value(host.list_scenarios()?)?,
        "get_scenario" => {
            let id = args["id"].as_i64().unwrap_or(0);
            serde_json::to_value(host.get_scenario(id)?)?
        }
        "save_scenario" => {
            let id = args["id"].as_i64();
            let sc: engine::model::Scenario = serde_json::from_value(args["scenario"].clone())?;
            json!({ "id": host.save_scenario(id, &sc)? })
        }
        "delete_scenario" => {
            host.delete_scenario(args["id"].as_i64().unwrap_or(0))?;
            json!({})
        }
        "preset_scenario" => {
            let name = args["name"].as_str().unwrap_or("pilot");
            serde_json::to_value(engine::preset(name))?
        }
        "auto_layout" => {
            let nodes = engine::auto_layout(
                args["metro_km"].as_f64().unwrap_or(20.0),
                args["n_dark"].as_u64().unwrap_or(2) as u32,
                args["n_store"].as_u64().unwrap_or(15) as u32,
                args["flagship"].as_bool().unwrap_or(true),
                args["orders_per_day"].as_f64().unwrap_or(1500.0),
            );
            serde_json::to_value(nodes)?
        }
        "run_simulation" => {
            let (run_id, result) = host.run_simulation(args["scenario_id"].as_i64().unwrap_or(0))?;
            json!({ "run_id": run_id, "result": result })
        }
        "list_runs" => serde_json::to_value(host.list_runs()?)?,
        "get_run" => serde_json::to_value(host.get_run(args["run_id"].as_i64().unwrap_or(0))?)?,
        "get_run_scenario" => {
            serde_json::to_value(host.get_run_scenario(args["run_id"].as_i64().unwrap_or(0))?)?
        }
        "get_frames" => {
            let frames = host.get_frames(
                args["run_id"].as_i64().unwrap_or(0),
                args["from_min"].as_u64().unwrap_or(0) as u32,
                args["to_min"].as_u64().unwrap_or(1440) as u32,
            )?;
            serde_json::to_value(frames)?
        }
        "delete_run" => {
            host.delete_run(args["run_id"].as_i64().unwrap_or(0))?;
            json!({})
        }
        "validate_math" => serde_json::to_value(engine::validate_math())?,
        "network_sweep" => {
            let sc = host.get_scenario(args["scenario_id"].as_i64().unwrap_or(0))?;
            serde_json::to_value(engine::network_sweep(
                &sc,
                args["n_from"].as_u64().unwrap_or(2) as u32,
                args["n_to"].as_u64().unwrap_or(10) as u32,
            ))?
        }
        "compare_partnerships" => {
            let sc = host.get_scenario(args["scenario_id"].as_i64().unwrap_or(0))?;
            serde_json::to_value(engine::compare_partnerships(&sc))?
        }
        "rank_stores" => {
            let sc = host.get_scenario(args["scenario_id"].as_i64().unwrap_or(0))?;
            serde_json::to_value(engine::rank_store_activations(
                &sc,
                args["take"].as_u64().unwrap_or(15) as usize,
            ))?
        }
        _ => anyhow::bail!("unknown command {cmd}"),
    };
    Ok(v)
}

#[tokio::main]
async fn main() {
    let dir = std::env::temp_dir().join("paratwin-dev");
    let host = Arc::new(Host::open(&dir).expect("open host"));
    host.seed_presets().expect("seed");
    let app = Router::new()
        .route("/rpc/:cmd", post(rpc))
        .layer(CorsLayer::permissive())
        .with_state(host);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await.unwrap();
    println!("ParaTwin dev server on http://127.0.0.1:8787");
    axum::serve(listener, app).await.unwrap();
}
