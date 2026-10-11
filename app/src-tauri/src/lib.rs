// The modules below are `pub` only so `examples/fixture_devices.rs` can reuse them
// (examples link the library like any other crate); the app itself never needs it.
pub mod account;
mod backup;
mod catalog;
pub mod cloud;
mod commands;
mod covers;
mod csv_io;
pub mod data_folder;
pub mod db;
mod device_reset;
pub mod error;
mod lan;
mod libraries;
mod links;
mod marks;
mod metadata;
pub mod ops;
mod people;
mod permissions;
mod requests;
mod search;
mod series;
mod settings;
mod sync;

use tauri::{Emitter, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use crate::account::ResetDeps;
use crate::cloud::events::EventsService;
use crate::covers::Covers;
use crate::db::Database;
use crate::device_reset::DataDir;
use crate::lan::identity::Identity;
use crate::lan::service::LanService;
use crate::links::Arrivals;
use crate::metadata::Lookup;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // First of all: a second launch (the system opening a link while the app runs) must be caught before
    // anything else starts. A second copy with its own data folder, to test the LAN (debug builds), is meant.
    #[cfg(desktop)]
    let builder = if cfg!(debug_assertions) && std::env::var_os("KOSMOTHEKA_DATA_DIR").is_some() {
        builder
    } else {
        builder.plugin(
            tauri_plugin_single_instance::Builder::new()
                .callback(|app, _arguments, _directory| links::show_window(app))
                .build(),
        )
    };
    let builder = builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init());
    #[cfg(mobile)]
    let builder = builder
        .plugin(tauri_plugin_barcode_scanner::init())
        .plugin(tauri_plugin_biometric::init());
    builder
        .setup(|app| {
            let data_dir = data_dir(app.path().app_data_dir()?)?;
            std::fs::create_dir_all(&data_dir)?;
            let database = Database::open(data_dir.join(data_folder::DATABASE_FILE))?;
            let lan = LanService::new(database.clone(), Identity::load_or_create(&data_dir)?);
            // Not worth failing the start over: the next rename tries again.
            let _ = lan::refresh_device_name(&mut *database.lock()?);
            let covers = Covers::new(data_dir.join("covers"))?;
            // The config's scope names the default folder; a data folder set with
            // KOSMOTHEKA_DATA_DIR keeps its covers elsewhere, and they must show too.
            app.asset_protocol_scope()
                .allow_directory(covers.dir(), false)?;
            app.manage(database.clone());
            app.manage(covers.clone());
            app.manage(Lookup::new()?);
            app.manage(lan.clone());
            app.manage(DataDir(data_dir.clone()));
            let reset_deps = ResetDeps::new(covers, lan.clone(), DataDir(data_dir));
            app.manage(reset_deps.clone());
            // Live notices from the cloud, while signed in (D75, milestone 7.3).
            let events = EventsService::new(app.handle().clone(), database.clone(), reset_deps);
            events.start_if_signed_in();
            app.manage(events);
            // Links the system hands over: the one that started the app, and those that come while it runs.
            app.manage(Arrivals::default());
            let link_handle = app.handle().clone();
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                links::receive(&link_handle, urls.iter().map(url::Url::as_str));
            }
            app.deep_link().on_open_url(move |event| {
                let urls = event.urls();
                links::receive(&link_handle, urls.iter().map(url::Url::as_str));
            });
            // A paired device that pushes something new: the UI reloads.
            let arrival_handle = app.handle().clone();
            lan.on_arrival(move || {
                let _ = arrival_handle.emit("library-changed", ());
            });
            // This device's own changes: after a short wait to gather a burst, the UI
            // syncs them out. Merges from other devices never wake it.
            let local_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    ops::local_change_burst(std::time::Duration::from_secs(2)).await;
                    let _ = local_handle.emit("local-changed", ());
                }
            });
            // Devices paired earlier can sync as soon as the app is open.
            let start_lan = lan.clone();
            tauri::async_runtime::spawn(async move {
                let _ = start_lan.start_if_paired().await;
            });
            // A device removed from its account while this app was closed learns it
            // in the background, as soon as it is open again (D81).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let reset = handle.state::<ResetDeps>();
                let result = account::refresh_account_on_start(&database, reset.context()).await;
                commands::emit_if_reset(&handle, &result);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_book,
            commands::list_books,
            commands::get_book,
            commands::update_edition,
            commands::delete_book,
            commands::add_copy,
            commands::update_copy,
            commands::delete_copy,
            commands::list_tags,
            commands::rename_tag,
            commands::delete_tag,
            commands::genres_of_kind,
            commands::request_book,
            commands::add_missing,
            commands::request_missing,
            commands::request_add_copy,
            commands::request_delete_copy,
            commands::request_delete_book,
            commands::book_history,
            commands::lookup_isbn,
            commands::search_works,
            commands::lookup_code,
            commands::get_settings,
            commands::save_settings,
            commands::covers_dir,
            commands::catalog_options,
            commands::export_backup,
            commands::export_device_backup,
            commands::restore_backup,
            commands::export_csv,
            commands::import_csv,
            commands::lan_status,
            commands::set_device_name,
            commands::open_pairing,
            commands::close_pairing,
            commands::pair_with,
            commands::sync_with,
            commands::forget_device,
            commands::check_isbn,
            commands::is_mobile,
            commands::welcome_seen,
            commands::set_welcome_seen,
            commands::app_lock_enabled,
            commands::set_app_lock,
            commands::app_lock_after,
            commands::set_app_lock_after,
            commands::list_libraries,
            commands::create_library,
            commands::rename_library,
            commands::set_active_library,
            commands::delete_library,
            commands::leave_library,
            commands::move_book,
            commands::library_devices,
            commands::sharing_overview,
            commands::set_sharing,
            commands::list_requests,
            commands::accept_request,
            commands::reject_request,
            commands::withdraw_request,
            commands::requested_book,
            commands::accept_request_edited,
            commands::request_notices,
            commands::dismiss_request_notices,
            commands::list_people,
            commands::start_roles,
            commands::known_people,
            commands::pairing_owners,
            commands::give_access,
            commands::work_marks,
            commands::set_mark,
            commands::claim_person,
            commands::claim_candidates,
            commands::set_role,
            commands::assign_device,
            commands::remove_device,
            commands::withdraw_person,
            commands::restore_person,
            commands::duplicate_library,
            commands::list_series,
            commands::get_series,
            commands::create_series,
            commands::update_series,
            commands::delete_series,
            commands::add_to_series,
            commands::remove_from_series,
            commands::reorder_series,
            commands::add_requirement,
            commands::remove_requirement,
            commands::account_status,
            commands::dismiss_account_notice,
            commands::set_account_server,
            commands::account_server_info,
            commands::register_account,
            commands::confirm_account_email,
            commands::resend_account_code,
            commands::sign_in,
            commands::sign_out,
            commands::forgot_password,
            commands::reset_password,
            commands::get_account,
            commands::update_account,
            commands::change_password,
            commands::delete_account,
            commands::account_devices,
            commands::remove_account_device,
            commands::restore_account_device,
            commands::two_factor_setup,
            commands::two_factor_enable,
            commands::two_factor_disable,
            commands::regenerate_backup_codes,
            commands::give_device_away,
            commands::cloud_status,
            commands::upload_to_cloud,
            commands::set_cloud_power,
            commands::sync_cloud,
            commands::cloud_other_owners,
            commands::answer_other_owner,
            commands::cloud_events_wake,
            commands::cloud_connection,
            commands::invite_status,
            commands::invite_to_library,
            commands::list_invitations,
            commands::revoke_invitation,
            commands::open_invitation,
            commands::open_arrived_link,
            commands::pending_invitation,
            commands::discard_pending_invitation,
            commands::accept_invitation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// In debug builds `KOSMOTHEKA_DATA_DIR` points a second instance at its own data, so
/// two copies of the app on one computer can sync with each other. Otherwise the
/// app's own folder, carried over from the old identifier the first time (D70).
fn data_dir(default: std::path::PathBuf) -> error::Result<std::path::PathBuf> {
    #[cfg(debug_assertions)]
    if let Some(dir) = std::env::var_os("KOSMOTHEKA_DATA_DIR") {
        return Ok(dir.into());
    }
    if let Some(legacy) = data_folder::legacy_dir(&default) {
        data_folder::copy_legacy_data(&default, &legacy)?;
    }
    Ok(default)
}
