mod errors;
mod screen_manager;

use crate::errors::CloudError;
use crate::screen_manager::stop_screen;
use axum;
use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::sync::Arc;
use tokio;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};

#[derive(Serialize, Deserialize, Clone)]
struct Server {
    server_id: String,
    server_name: String,
    port: u16,
    max_player: u16,
}

#[derive(Serialize, Deserialize, Clone)]
struct PortAvailability {
    port: u16,
    is_available: bool,
}

#[derive(Serialize, Deserialize, Clone)]
struct Daemon {
    server_list: Vec<Server>,
    port_list: Vec<PortAvailability>,
}

impl Default for Daemon {
    fn default() -> Self {
        let initialized_ports: Vec<PortAvailability> = (25570..25590)
            .into_iter()
            .map(|x| PortAvailability {
                port: x,
                is_available: true,
            })
            .collect();
        Self {
            server_list: Vec::new(),
            port_list: initialized_ports,
        }
    }
}

#[derive(Clone)]
struct AppState {
    daemon: Arc<Mutex<Daemon>>,
    shutdown: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

fn init_cloud() -> Result<(), CloudError> {
    let status = create_dir_all("running/static");
    if status.is_err() {
        return Err(CloudError::FileError);
    };
    let status = create_dir_all("running/disposable");
    if status.is_err() {
        return Err(CloudError::FileError);
    };
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), CloudError> {
    println!("Starting AesirCloud...");

    println!("Initializing cloud...");
    let cloud_status = init_cloud();
    cloud_status.as_ref().expect("Shutting down: ");
    if cloud_status.is_err() {
        return Err(CloudError::FatalError);
    };

    let daemon = Arc::new(Mutex::new(Daemon::default()));
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let app_state = AppState {
        daemon,
        shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
    };

    let app = Router::new()
        .route("/", get(test_route))
        .route("/stop/{name}", delete(stop_instance))
        .route("/shutdown", post(shutdown))
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();

    println!("Done !");
    println!("Listening on port: 3001");
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
            println!("Shutting down...");
        })
        .await
        .unwrap();
    Ok(())
}

async fn shutdown(State(state): State<AppState>) {
    if let Some(tx) = state.shutdown.lock().await.take() {
        let _ = tx.send(());
    }
}

async fn test_route(State(state): State<AppState>) -> Json<Vec<PortAvailability>> {
    let guard = state.daemon.lock().await;
    Json(guard.port_list.clone())
}

async fn stop_instance(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Vec<Server>> {
    let guard = state.daemon.lock().await;
    stop_screen(name).expect("Error in \'screen\' command !");
    Json(guard.server_list.clone())
}
