pub mod activity_log;
pub mod apparmor;
pub mod blueprints;
pub mod commands;
pub mod config;
pub mod desktop;
pub mod dirs;
pub mod download;
pub mod error;
pub mod executor;
pub mod fleet;
pub mod notifications;
pub mod polkit;
pub mod state;
pub mod tasks;
pub mod variables;
pub mod vault;

use std::sync::Arc;
use tokio::sync::Mutex;

use activity_log::ActivityLog;
use apparmor::template_manager::ProfileTemplateManager;
use blueprints::manager::BlueprintManager;
use config::manager::ConfigManager;
use fleet::manager::FleetManager;
use tasks::registry::create_registry;
use vault::Vault;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = ConfigManager::load().unwrap_or_default();
    let registry = create_registry();
    let fleet_manager = FleetManager::load()
        .expect("Failed to initialize fleet manager");
    let blueprint_manager = BlueprintManager::load()
        .expect("Failed to initialize blueprint manager");
    let profile_templates = ProfileTemplateManager::load();
    let vault = Vault::new();

    let activity_log_path = dirs::config_dir().join("activity_log.json");
    let activity_log = ActivityLog::new(activity_log_path, 500);

    let app_state = AppState {
        registry: Arc::new(registry),
        config: Arc::new(Mutex::new(config)),
        monitor_running: Arc::new(Mutex::new(false)),
        fleet_manager: Arc::new(Mutex::new(fleet_manager)),
        blueprint_manager: Arc::new(Mutex::new(blueprint_manager)),
        profile_templates: Arc::new(profile_templates),
        vault: Arc::new(Mutex::new(vault)),
        activity_log: Arc::new(Mutex::new(activity_log)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::activity::get_activity_log,
            commands::activity::clear_activity_log,
            commands::tasks::list_tasks,
            commands::tasks::list_tasks_for_node,
            commands::tasks::detect_all_states,
            commands::tasks::execute_task,
            commands::tasks::uninstall_task,
            commands::tasks::get_installation_states,
            commands::tasks::check_task_updates,
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
            commands::apparmor::list_profile_templates,
            commands::apparmor::get_profile_template,
            commands::apparmor::preview_profile,
            commands::apparmor::install_profile_template,
            commands::apparmor::uninstall_profile_template,
            commands::apparmor::sync_installed_profiles,
            commands::apparmor::get_profile_template_config,
            commands::apparmor::set_profile_template_config,
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
            commands::fleet::poll_all_node_statuses,
            commands::fleet::discover_ssh_hosts_cmd,
            commands::fleet::open_node_terminal,
            commands::blueprints::list_blueprints,
            commands::blueprints::get_blueprint,
            commands::blueprints::create_blueprint,
            commands::blueprints::update_blueprint,
            commands::blueprints::remove_blueprint,
            commands::blueprints::apply_blueprint,
            commands::blueprints::apply_blueprint_bulk,
            commands::blueprints::uninstall_blueprint,
            commands::blueprints::uninstall_blueprint_bulk,
            commands::blueprints::dry_run_blueprint,
            commands::blueprints::clone_blueprint,
            commands::blueprints::create_blank_blueprint,
            commands::blueprints::delete_blueprint,
            commands::blueprints::assign_blueprint,
            commands::blueprints::unassign_blueprint,
            commands::blueprints::import_blueprint_package,
            commands::blueprints::export_blueprint_package,
            commands::ssh::test_ssh_connection,
            commands::ssh::list_ssh_keys,
            commands::ssh::generate_ssh_key,
            commands::ssh::deploy_ssh_key,
            commands::yubikey::detect_yubikeys,
            commands::yubikey::list_fido2_credentials,
            commands::yubikey::generate_fido2_ssh_key,
            commands::yubikey::is_fido2_supported,
            commands::vault::vault_exists,
            commands::vault::vault_is_unlocked,
            commands::vault::vault_create,
            commands::vault::vault_unlock,
            commands::vault::vault_lock,
            commands::vault::vault_get,
            commands::vault::vault_set,
            commands::vault::vault_remove,
            commands::vault::vault_list_keys,
            commands::vault::vault_change_password,
            commands::variables::get_shared_variables,
            commands::variables::set_shared_variable,
            commands::variables::remove_shared_variable,
            commands::variables::get_node_variable_overrides,
            commands::variables::set_node_variable_override,
            commands::variables::remove_node_variable_override,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
