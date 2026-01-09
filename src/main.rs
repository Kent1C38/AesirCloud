mod errors;
mod file_downloader;
mod heartbeat;
mod instance;
mod minecraft_version;
mod screen_manager;
mod loader;

use crate::errors::CloudError;
use crate::instance::{Instance, create_instance, start_instance_status, stop_instance};
use axum;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{create_dir_all, write};
use std::process::exit;
use std::sync::Arc;
use tokio;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::{Mutex, oneshot};
use crate::heartbeat::heartbeat_handler;

const PORT_RANGE: std::ops::Range<u16> = 25570..2999;

#[derive(Serialize, Deserialize, Clone)]
struct PortAvailability {
    port: u16,
    is_available: bool,
}

#[derive(Serialize, Deserialize)]
struct PersistentState {
    server_list: Vec<Instance>,
}

struct Daemon {
    server_list: Vec<Arc<Mutex<Instance>>>,
    used_ports: HashSet<u16>,
}

impl Default for Daemon {
    fn default() -> Self {
        Self {
            server_list: Vec::new(),
            used_ports: HashSet::new(),
        }
    }
}

impl Daemon {
    pub async fn get_instance(&self, server_id: &str) -> Option<Arc<Mutex<Instance>>> {
        for inst in &self.server_list {
            let guard = inst.lock().await;
            if guard.server_id == server_id {
                return Some(inst.clone())
            }
        }
        None
    }

    fn from_persistent(state: PersistentState) -> Self {
        let server_list = state
            .server_list
            .into_iter()
            .map(|inst| Arc::new(Mutex::new(inst)))
            .collect::<Vec<_>>();

        let used_ports = server_list
            .iter()
            .map(|inst| {
                let inst = futures::executor::block_on(inst.lock());
                inst.port
            })
            .collect::<HashSet<_>>();

        Self {
            server_list,
            used_ports,
        }
    }

    async fn save(&self) -> Result<(), CloudError> {
        let mut persistent_instances = Vec::new();
        for inst in &self.server_list {
            let guard = inst.lock().await;
            if guard.is_persistent {
                persistent_instances.push(guard.clone());
            }
        }

        let state = PersistentState {
            server_list: persistent_instances
        };

        let json = serde_json::to_string_pretty(&state)
            .map_err(|_| CloudError::JSONError)?;
        write("state.json", json).map_err(|_| CloudError::FileError)?;
        Ok(())
    }

    fn load_or_default() -> Self {
        match std::fs::read_to_string("state.json") {
            Ok(content) => match serde_json::from_str::<PersistentState>(&content) {
                Ok(state) => Self::from_persistent(state),
                Err(_) => Self::default(),
            },
            Err(_) => Self::default(),
        }
    }

    fn allocate_port(&mut self) -> Option<u16> {
        for port in PORT_RANGE {
            if self.used_ports.contains(&port) {
                continue;
            }

            if std::net::TcpListener::bind(("0.0.0.0", port)).is_ok() {
                self.used_ports.insert(port);
                return Some(port);
            }
        }
        None
    }

    fn free_port(&mut self, port: u16) {
        self.used_ports.remove(&port);
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

    let daemon = Arc::new(Mutex::new(Daemon::load_or_default()));
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let app_state = AppState {
        daemon: daemon.clone(),
        shutdown: Arc::new(Mutex::new(Some(shutdown_tx))),
    };

    let daemon_for_sig = daemon.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("ctrl_c failed");
        println!("SIGINT received");
        let guard = daemon_for_sig.lock().await;
        if let Err(_) = guard.save().await {
            eprintln!("Failed to save state !");
        }
        println!("State saved !");
        println!("Goodbye !");
        exit(0);
    });

    let app = Router::new()
        .route("/", get(test_route))
        .route("/stop/{name}", delete(stop_instance))
        .route("/start/{name}", post(start_instance_status))
        .route("/shutdown", post(shutdown))
        .route("/register", post(create_instance))
        .route("/heartbeat/{name}", post(heartbeat_handler))
        .with_state(app_state);

    let listener = TcpListener::bind("0.0.0.0:3001").await.unwrap();

    println!("Done !");
    println!("Listening on port: 3001");
    let daemon_shutdown = daemon.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown({
            let shutdown_rx = shutdown_rx;
            async move {
                shutdown_rx.await.ok();

                println!("HTTP shutdown requested");
                let guard = daemon_shutdown.lock().await;
                if let Err(_) = guard.save().await {
                    eprintln!("Failed to save state !");
                }
                println!("State saved !");
                println!("Goodbye !");
            }
        })
        .await
        .unwrap();
    Ok(())
}

async fn shutdown(State(state): State<AppState>) -> impl IntoResponse {
    if let Some(tx) = state.shutdown.lock().await.take() {
        let _ = tx.send(());
    }
    StatusCode::OK
}

async fn test_route(State(state): State<AppState>) -> Json<Vec<Instance>> {
    let guard = state.daemon.lock().await;
    let mut instances = Vec::new();
    for inst in &guard.server_list {
        let inst_guard = inst.lock().await;
        instances.push(inst_guard.clone())
    }
    Json(instances)
}
