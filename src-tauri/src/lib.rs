pub mod blueprints;
pub mod commands;
pub mod config;
pub mod error;
pub mod executor;
pub mod fleet;
pub mod tasks;
pub mod apparmor;
pub mod polkit;
pub mod state;

use std::sync::Arc;
use tokio::sync::Mutex;

use blueprints::manager::BlueprintManager;
use config::manager::ConfigManager;
use fleet::manager::FleetManager;
use tasks::registry::create_registry;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = ConfigManager::load().unwrap_or_default();
    let registry = create_registry();
    let fleet_manager = FleetManager::load()
        .expect("Failed to initialize fleet manager");
    let blueprint_manager = BlueprintManager::load()
        .expect("Failed to initialize blueprint manager");

    let app_state = AppState {
        registry: Arc::new(registry),
        config: Arc::new(Mutex::new(config)),
        monitor_running: Arc::new(Mutex::new(false)),
        fleet_manager: Arc::new(Mutex::new(fleet_manager)),
        blueprint_manager: Arc::new(Mutex::new(blueprint_manager)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::tasks::list_tasks,
            commands::tasks::detect_all_states,
            commands::tasks::execute_task,
            commands::apparmor::get_denial_logs,
            commands::apparmor::get_profiles,
            commands::apparmor::get_profile_detail,
            commands::apparmor::apply_permission_rules,
            commands::apparmor::start_log_monitor,
            commands::apparmor::stop_log_monitor,
            commands::apparmor::consolidate_rules,
            commands::apparmor::apply_permission_rules_batch,
            commands::apparmor::consolidate_profile_rules,
            commands::apparmor::rewrite_profile_rules,
            commands::config::get_config,
            commands::config::set_config,
            commands::system::get_system_info,
            commands::system::check_polkit_auth,
            commands::system::is_polkit_policy_installed,
            commands::system::is_apparmor_helper_installed,
            commands::system::install_polkit_policy,
            commands::fleet::list_nodes,
            commands::fleet::get_node,
            commands::fleet::add_node,
            commands::fleet::update_node,
            commands::fleet::remove_node,
            commands::fleet::get_node_status,
            commands::blueprints::list_blueprints,
            commands::blueprints::get_blueprint,
            commands::blueprints::create_blueprint,
            commands::blueprints::update_blueprint,
            commands::blueprints::remove_blueprint,
            commands::blueprints::apply_blueprint,
            commands::blueprints::clone_blueprint,
            commands::blueprints::create_blank_blueprint,
            commands::blueprints::delete_blueprint,
            commands::blueprints::assign_blueprint,
            commands::blueprints::unassign_blueprint,
            commands::ssh::test_ssh_connection,
            commands::ssh::list_ssh_keys,
            commands::ssh::generate_ssh_key,
            commands::ssh::deploy_ssh_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
