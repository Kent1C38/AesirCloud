mod errors;
mod screen_manager;
mod file_downloader;

use crate::errors::CloudError;
use crate::screen_manager::{stop_screen, JavaVersion};
use axum;
use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::fs::create_dir_all;
use std::sync::Arc;
use axum::response::IntoResponse;
use reqwest::StatusCode;
use tokio;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, oneshot};

#[derive(Deserialize, Serialize, Clone)]
enum MinecraftVersion {
    V1_21,
}

#[derive(Deserialize, Serialize, Clone)]
enum MineacrftLoader {
    Paper(MinecraftVersion),
    ThunderStorm(MinecraftVersion),
}
impl MineacrftLoader {
    pub fn download_url(&self) -> &'static str {
        match self {
            MineacrftLoader::Paper(MinecraftVersion::V1_21) => "https://fill-data.papermc.io/v1/objects/a61a0585e203688f606ca3a649760b8ba71efca01a4af7687db5e41408ee27aa/paper-1.21.10-117.jar",
            MineacrftLoader::ThunderStorm(MinecraftVersion::V1_21) => ""
        }
    }

    pub fn get_java_version(&self) -> JavaVersion {
        match self {
            MineacrftLoader::Paper(MinecraftVersion::V1_21) => JavaVersion::J21,
            MineacrftLoader::ThunderStorm(MinecraftVersion::V1_21) => JavaVersion::J25
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Instance {
    server_id: String,
    server_name: String,
    loader: MineacrftLoader,
    folder: String,
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
    server_list: Vec<Instance>,
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
    create_dir_all("running/static").map_err(|_| CloudError::FileError)?;
    create_dir_all("templates").map_err(|_| CloudError::FileError)?;
    create_dir_all("versions").map_err(|_| CloudError::FileError)?;
    create_dir_all("running/disposable").map_err(|_| CloudError::FileError)
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
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let app_state = AppState {
        daemon,
        shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
    };

    let app = Router::new()
        .route("/", get(test_route))
        .route("/stop/{name}", delete(stop_instance))
        .route("/shutdown", post(shutdown))
        .route("/start", post(start_static_instance))
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
) -> Json<Vec<Instance>> {
    let guard = state.daemon.lock().await;
    stop_screen(name).expect("Error in \'screen\' command !");
    Json(guard.server_list.clone())
}

async fn start_static_instance(
    State(state): State<AppState>,
    Json(request): Json<Instance>
) -> impl IntoResponse {
    match do_start_static_instance(state.daemon.clone(), request).await {
        Ok(_) => (StatusCode::OK, "Instance Started").into_response(),
        Err(err) => {
            eprintln!("Error starting instance: {:?}", err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Error starting instance").into_response()
        }
    }
}

async fn do_start_static_instance(
    daemon: Arc<Mutex<Daemon>>,
    request: Instance
) -> Result<(), CloudError> {
    let mut daemon_guard = daemon.lock().await;

    if let Some(port) = daemon_guard.port_list.iter_mut().find(|p| p.port == request.port) {
        if !port.is_available { return Err(CloudError::UnavailablePort) }
        port.is_available = false;
    } else {
        return Err(CloudError::UnavailablePort)
    }
    daemon_guard.server_list.push(request.clone());
    drop(daemon_guard);
    screen_manager::start_screen(request).await?;
    Ok(())
}
