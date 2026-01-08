use std::fs;
use crate::errors::CloudError;
use crate::minecraft_loader::MinecraftLoader;
use crate::screen_manager::{start_screen, stop_screen};
use crate::{AppState, Daemon};
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, Clone)]
pub struct Instance {
    pub server_id: String,
    pub server_name: String,
    pub is_persistent: bool,
    pub loader: MinecraftLoader,
    pub folder: String,
    pub port: u16,
    pub max_player: u16,
    pub started: bool,
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

    if guard
        .server_list
        .iter()
        .any(|x| x.server_id == instance.server_id)
    {
        return Err(CloudError::InstanceAlreadyExists);
    }

    let dir_path = format!(
        "running/{}/{}",
        if instance.is_persistent {"static"} else {"disposable"},
        instance.server_id
    );

    if let Err(e) = fs::create_dir_all(&dir_path) {
        eprintln!("Failed to create directory {}: {}", dir_path, e);
        return Err(CloudError::FileError);
    }

    guard.server_list.push(instance);
    Ok(())
}

pub async fn start_instance_status(
    State(state): State<AppState>,
    Path(server_id): Path<String>,
) -> impl IntoResponse {
    let instance_opt = {
        let guard = state.daemon.lock().await;
        guard.get_instance(&server_id).cloned()
    };

    if let Some(instance) = instance_opt {
        start_instance(instance).await.into_response()
    } else {
        (StatusCode::NOT_FOUND, "Could not find this instance !").into_response()
    }
}

async fn start_instance(instance: Instance) -> (StatusCode, String) {

    let loader = &instance.loader;

    if !loader.is_installed() {
        if let Err(_) = loader.install().await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error while retrieving the minecraft loader !".to_string()
            );
        }
        println!("Downloaded new minecraft loader");
    }

    if let Err(_) = start_screen(instance).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Error while starting screen".to_string())
    }

    (StatusCode::OK, "Server started".to_string())
}

// pub async fn start_static_instance(
//     State(state): State<AppState>,
//     Json(request): Json<Instance>,
// ) -> impl IntoResponse {
//     match do_start_static_instance(state.daemon.clone(), request).await {
//         Ok(_) => (StatusCode::OK, "Instance Started").into_response(),
//         Err(err) => {
//             eprintln!("Error starting instance: {:?}", err);
//             (StatusCode::INTERNAL_SERVER_ERROR, "Error starting instance").into_response()
//         }
//     }
// }

pub async fn stop_instance(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Json<Vec<Instance>> {
    let guard = state.daemon.lock().await;
    stop_screen(name).expect("Error in \'screen\' command !");
    Json(guard.server_list.clone())
}

// async fn do_start_static_instance(
//     daemon: Arc<Mutex<Daemon>>,
//     request: Instance,
// ) -> Result<(), CloudError> {
//     let mut daemon_guard = daemon.lock().await;
//
//     if let Some(port) = daemon_guard
//         .port_list
//         .iter_mut()
//         .find(|p| p.port == request.port)
//     {
//         if !port.is_available {
//             return Err(CloudError::UnavailablePort);
//         }
//         port.is_available = false;
//     } else {
//         return Err(CloudError::UnavailablePort);
//     }
//     daemon_guard.server_list.push(request.clone());
//     drop(daemon_guard);
//     screen_manager::start_screen(request).await?;
//     Ok(())
// }
