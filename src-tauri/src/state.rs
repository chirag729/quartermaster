use std::sync::Arc;
use tokio::sync::Mutex;

use crate::blueprints::manager::BlueprintManager;
use crate::config::manager::ConfigManager;
use crate::fleet::manager::FleetManager;
use crate::tasks::registry::TaskRegistry;

pub struct AppState {
    pub registry: Arc<TaskRegistry>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub monitor_running: Arc<Mutex<bool>>,
    pub fleet_manager: Arc<Mutex<FleetManager>>,
    pub blueprint_manager: Arc<Mutex<BlueprintManager>>,
}
