use std::sync::Arc;
use tokio::sync::Mutex;

use crate::activity_log::ActivityLog;
use crate::apparmor::template_manager::ProfileTemplateManager;
use crate::blueprints::manager::BlueprintManager;
use crate::config::manager::ConfigManager;
use crate::fleet::manager::FleetManager;
use crate::tasks::registry::TaskRegistry;
use crate::vault::Vault;

pub struct AppState {
    pub registry: Arc<TaskRegistry>,
    pub config: Arc<Mutex<ConfigManager>>,
    pub monitor_running: Arc<Mutex<bool>>,
    pub fleet_manager: Arc<Mutex<FleetManager>>,
    pub blueprint_manager: Arc<Mutex<BlueprintManager>>,
    pub profile_templates: Arc<ProfileTemplateManager>,
    pub vault: Arc<Mutex<Vault>>,
    pub activity_log: Arc<Mutex<ActivityLog>>,
}
