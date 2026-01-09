use crate::instance::Instance;
use crate::screen_manager::stop_screen;
use crate::{AppState, Daemon};
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use reqwest::StatusCode;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;
use tokio::time::interval;

pub async fn heartbeat_handler(
	State(state): State<AppState>,
	Path(name): Path<String>,
) -> impl IntoResponse {
	let instance_arc = {
		let guard = state.daemon.lock().await;
		guard.get_instance(&name).await
	};

	let Some(instance_arc) = instance_arc else {
		return (
			StatusCode::UNAUTHORIZED,
			"Unrecognized server, are you sure that you're supposed to be here ?",
		).into_response();
	};

	let (should_start_heartbeat, should_accept_beat) = {
		let instance = instance_arc.lock().await;
		(
			!instance.heartbeat_started && instance.started,
			instance.started,
		)
	};

	if should_start_heartbeat {
		start_heartbeat_check(instance_arc.clone(), state.daemon.clone()).await;

		let mut instance = instance_arc.lock().await;
		instance.last_heartbeat = now();

		return (StatusCode::OK, "Beat started").into_response();
	}

	if should_accept_beat {
		let mut instance = instance_arc.lock().await;
		instance.last_heartbeat = now();

		return (StatusCode::OK, "Beat").into_response();
	}

	if let Err(_) = stop_screen(instance_arc).await {
		(
			StatusCode::INTERNAL_SERVER_ERROR,
			"Could not stop this instance, are you sure it was up ?",
		).into_response()
	} else {
		(StatusCode::CONFLICT, "Should be off, shutting down").into_response()
	}
}

fn now() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backward ??")
		.as_secs()
}


pub async fn start_heartbeat_check(instance: Arc<Mutex<Instance>>, daemon: Arc<Mutex<Daemon>>) {
	tokio::spawn(async move {
		{
			let mut guard = instance.lock().await;
			guard.heartbeat_started = true;
		}
		let mut interval = interval(Duration::from_secs(10));
		loop {
			interval.tick().await;
			let alive = {
				let inst_guard = instance.lock().await;
				is_server_alive(&inst_guard)
			};

			if !alive {
				let server_id = {
					let inst_guard = instance.lock().await;
					inst_guard.server_id.clone()
				};
				println!("Server {} seems down, unregistering...", server_id);
				let guard = daemon.lock().await;
				if let Some(inst_arc) = guard.get_instance(&server_id).await {
					if let Err(_) = stop_screen(inst_arc.clone()).await {
						eprintln!("Error stopping server {}", server_id)
					}
					let mut inst_guard = inst_arc.lock().await;
					inst_guard.started = false
				}
				break
			}
		}
	});
}

pub fn is_server_alive(instance: &Instance) -> bool {
	now() - instance.last_heartbeat < 10
}
