//! Builds one data folder per fixture device, signed in to the dev server and
//! synced, so each can be opened in the real app (docs/development/fixtures.md).
//!
//! cargo run -p kosmotheka --example fixture_devices -- <cast.json> <out dir>
//!
//! `cast.json` is written by `make fixtures` in `server/`. Needs the dev server up.

use std::path::Path;
use std::process::ExitCode;

use kosmotheka_lib::account::{self, AccountUser};
use kosmotheka_lib::cloud::{self, client::HttpCloud};
use kosmotheka_lib::data_folder::DATABASE_FILE;
use kosmotheka_lib::db::Database;
use kosmotheka_lib::ops;
use serde::Deserialize;

#[derive(Deserialize)]
struct Cast {
    devices: Vec<Device>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    folder: String,
    character: String,
    email: String,
    user: User,
    device_id: String,
    signing_seed: String,
    token: String,
    server: String,
}

#[derive(Deserialize)]
struct User {
    id: String,
    name: String,
    locale: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [cast, out] = args.as_slice() else {
        eprintln!("usage: fixture_devices <cast.json> <out dir>");
        return ExitCode::FAILURE;
    };
    match run(Path::new(cast), Path::new(out)).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cast: &Path, out: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(cast).map_err(|e| format!("{}: {e}", cast.display()))?;
    let cast: Cast = serde_json::from_str(&text).map_err(|e| format!("cast.json: {e}"))?;
    for device in &cast.devices {
        build(device, out).await?;
    }
    println!(
        "\nOpen one with: KOSMOTHEKA_DATA_DIR={}/<folder> target/debug/kosmotheka",
        out.display()
    );
    Ok(())
}

async fn build(device: &Device, out: &Path) -> Result<(), String> {
    let dir = out.join(&device.folder);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let database = Database::open(dir.join(DATABASE_FILE)).map_err(|e| e.to_string())?;
    {
        let conn = database.lock().map_err(|e| e.to_string())?;
        // Identity first: nothing is recorded yet, so every operation carries it.
        let seed = decode_seed(&device.signing_seed)?;
        ops::set_identity(&conn, &device.device_id, seed).map_err(|e| e.to_string())?;
        ops::write_state(&conn, account::ACCOUNT_SERVER, Some(&device.server))
            .map_err(|e| e.to_string())?;
        let user = AccountUser {
            id: device.user.id.clone(),
            email: device.email.clone(),
            name: device.user.name.clone(),
            locale: device.user.locale.clone(),
        };
        account::store_session(&conn, &device.token, &user, &device.email)
            .map_err(|e| e.to_string())?;
    }

    println!(
        "\n{} ({}) -> {}",
        device.character,
        device.folder,
        dir.display()
    );
    let api = HttpCloud::connect(&database).map_err(|e| e.to_string())?;
    let reports = cloud::sync_all(&database, &api)
        .await
        .map_err(|e| format!("{}: sync failed: {e}", device.folder))?;
    if reports.is_empty() {
        println!("  sync_all: no libraries (nothing the account may see)");
    }
    for report in reports {
        println!(
            "  library {} cloud {} -> {:?}: received {}, sent {}, rejected {}",
            report.library,
            report.cloud_library.as_deref().unwrap_or("-"),
            report.outcome,
            report.received,
            report.sent,
            report.rejected
        );
    }
    Ok(())
}

fn decode_seed(hex: &str) -> Result<[u8; 32], String> {
    if hex.len() != 64 || !hex.is_ascii() {
        return Err("the signing seed must be 64 hex characters".to_owned());
    }
    let mut seed = [0u8; 32];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|e| e.to_string())?;
    }
    Ok(seed)
}
