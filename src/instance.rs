use crate::errors::CloudError;
use crate::screen_manager::{start_screen, stop_screen};
use crate::{AppState, Daemon};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::loader::build_loader;
use crate::loader::config::LoaderConfig;

#[derive(Serialize, Deserialize, Clone)]
pub struct Instance {
    pub server_id: String,
    pub server_name: String,
    pub is_persistent: bool,
    pub loader: LoaderConfig,
    pub port: u16,
    pub max_player: u16,
    pub started: bool,
    pub heartbeat_started: bool,
    pub last_heartbeat: u64,
}

pub async fn create_instance(
    State(state): State<AppState>,
    Json(request): Json<Instance>,
) -> impl IntoResponse {
    match register_instance(state.daemon.clone(), request).await {
        Ok(_) => (StatusCode::CREATED, "Instance successfully registered").into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Could not register instance",
        )
            .into_response(),
    }
}

async fn register_instance(
    daemon: Arc<Mutex<Daemon>>,
    instance: Instance,
) -> Result<(), CloudError> {
    let mut guard = daemon.lock().await;

    for inst in &guard.server_list {
        let inst_guard = inst.lock().await;
        if inst_guard.server_id == instance.server_id {
            return Err(CloudError::InstanceAlreadyExists);
        }
    }

    let dir_path = format!(
        "running/{}/{}",
        if instance.is_persistent {
            "static"
        } else {
            "disposable"
        },
        instance.server_id
    );

    if let Err(e) = fs::create_dir_all(&dir_path) {
        eprintln!("Failed to create directory {}: {}", dir_path, e);
        return Err(CloudError::FileError);
    }

    let eula_path = format!("{}/{}", dir_path, "eula.txt");
    let mut eula = File::create(eula_path).map_err(|_| CloudError::FileError)?;
    eula.write("eula=true".as_bytes()).map_err(|_| CloudError::FileError)?;

    let config_path = format!("{}/{}", dir_path, "aesir.config");
    let mut config = File::create(config_path).map_err(|_| CloudError::FileError)?;
    config.write(format!("server_id={}", instance.server_id).as_bytes()).map_err(|_| CloudError::FileError)?;

    let properties_path = format!("{}/{}", dir_path, "server.properties");
    let mut properties = File::create(properties_path).map_err(|_| CloudError::FileError)?;
    properties.write(format!("max-players={}\nserver-port={}", instance.max_player, instance.port).as_bytes()).map_err(|_| CloudError::FileError)?;

    guard.server_list.push(Arc::new(Mutex::new(instance)));
    Ok(())
}

pub async fn start_instance_status(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
    let instance_opt = {
        let guard = state.daemon.lock().await;
        guard.get_instance(&server_id).await.clone()
    };

    if let Some(instance_arc) = instance_opt {
        start_instance(instance_arc).await.into_response()
    } else {
        (StatusCode::NOT_FOUND, "Could not find this instance").into_response()
    }
}

async fn start_instance(inst_arc: Arc<Mutex<Instance>>) -> (StatusCode, String) {
    let mut instance = inst_arc.lock().await;
    let loader = build_loader(&instance.loader);

    if !loader.is_installed() {
        if let Err(_) = loader.install().await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error while retrieving the minecraft loader !".to_string(),
            );
        }
        println!("Downloaded new minecraft loader");
    }

    if let Err(_) = start_screen(instance.clone()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Error while starting screen".to_string(),
        );
    }
    instance.started = true;
    (StatusCode::OK, "Server started".to_string())
}

pub async fn stop_instance(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
    let instance_opt = {
        let guard = state.daemon.lock().await;
        guard.get_instance(&server_id).await.clone()
    };

    if let Some(inst_arc) = instance_opt {
        if let Err(_) = stop_screen(inst_arc).await {
            (StatusCode::INTERNAL_SERVER_ERROR, "Error in 'screen' command !").into_response()
        } else {
            (StatusCode::OK, "Successfully stopped screen").into_response()
        }
    } else {
        (StatusCode::NOT_FOUND, "Could not find this instance").into_response()
    }
}
