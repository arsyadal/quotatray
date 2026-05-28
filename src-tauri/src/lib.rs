use chrono::{Datelike, TimeZone, Utc};
use keyring::Entry;
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::PathBuf, time::Duration};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, State,
};
use uuid::Uuid;

const KEYCHAIN_SERVICE_PREFIX: &str = "app.quotatray.provider";
const POLL_INTERVAL_SECONDS: u64 = 15 * 60;

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    http: Client,
}

#[derive(Debug, Clone)]
struct ProviderRow {
    id: String,
    provider_type: String,
    display_name: String,
    auth_type: String,
    keychain_service: Option<String>,
    keychain_account: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderView {
    id: String,
    provider_type: String,
    display_name: String,
    status: String,
    used: Option<f64>,
    #[serde(rename = "limit")]
    limit_value: Option<f64>,
    remaining: Option<f64>,
    percentage: Option<f64>,
    unit: String,
    reset_at: Option<String>,
    last_refresh_at: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DashboardPayload {
    providers: Vec<ProviderView>,
    global_status: String,
}

#[derive(Debug, Clone)]
struct UsageSnapshot {
    status: String,
    used: Option<f64>,
    limit_value: Option<f64>,
    remaining: Option<f64>,
    percentage: Option<f64>,
    unit: String,
    reset_at: Option<String>,
    message: Option<String>,
    raw_json: Option<String>,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterCreditsResponse {
    data: Option<OpenRouterCreditsData>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterCreditsData {
    total_credits: Option<f64>,
    total_usage: Option<f64>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            fs::create_dir_all(&app_data)?;
            let db_path = app_data.join("quotatray.sqlite3");
            init_db(&db_path).map_err(Box::<dyn std::error::Error>::from)?;

            let state = AppState {
                db_path: db_path.clone(),
                http: Client::builder()
                    .timeout(Duration::from_secs(20))
                    .user_agent("QuotaTray/0.1")
                    .build()?,
            };
            app.manage(state.clone());
            setup_tray(app.handle())?;
            start_scheduler(app.handle().clone(), state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_dashboard,
            add_mock_provider,
            add_codex_provider,
            add_opencode_go_provider,
            add_openrouter_provider,
            add_openai_provider,
            refresh_all,
            refresh_provider,
            remove_provider,
        ])
        .run(tauri::generate_context!())
        .expect("error while running QuotaTray");
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "Show QuotaTray").build(app)?;
    let refresh = MenuItemBuilder::with_id("refresh", "Refresh All").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&show, &refresh, &quit])
        .build()?;
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("QuotaTray — local quota cockpit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "refresh" => {
                if let Some(state) = app.try_state::<AppState>() {
                    let app_handle = app.clone();
                    let state = state.inner().clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = refresh_all_internal(&state).await;
                        update_tray_tooltip(&app_handle, &state);
                    });
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(&tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn start_scheduler(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(POLL_INTERVAL_SECONDS));
        loop {
            interval.tick().await;
            interval.tick().await;
            let _ = refresh_all_internal(&state).await;
            update_tray_tooltip(&app, &state);
        }
    });
}

#[tauri::command]
fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardPayload, String> {
    let conn = open_db(&state.db_path)?;
    let providers = load_provider_views(&conn)?;
    let global_status = compute_global_status(&providers);
    Ok(DashboardPayload {
        providers,
        global_status,
    })
}

#[tauri::command]
fn add_mock_provider(state: State<'_, AppState>) -> Result<(), String> {
    let conn = open_db(&state.db_path)?;
    let now = now_iso();
    conn.execute(
        "INSERT OR IGNORE INTO providers
        (id, provider_type, display_name, auth_type, keychain_service, keychain_account, enabled, created_at, updated_at)
        VALUES (?1, 'mock', 'Mock Provider', 'local', NULL, NULL, 1, ?2, ?2)",
        params!["mock-default", now],
    )
    .map_err(db_error)?;

    insert_snapshot(&conn, "mock-default", mock_snapshot())?;
    Ok(())
}

#[tauri::command]
async fn add_opencode_go_provider(state: State<'_, AppState>) -> Result<(), String> {
    let conn = open_db(&state.db_path)?;
    let now = now_iso();
    conn.execute(
        "INSERT OR IGNORE INTO providers
        (id, provider_type, display_name, auth_type, keychain_service, keychain_account, enabled, created_at, updated_at)
        VALUES (?1, 'opencode_go', 'OpenCode Go', 'local', NULL, NULL, 1, ?2, ?2)",
        params!["opencode-go-local", now],
    )
    .map_err(db_error)?;

    let row = ProviderRow {
        id: "opencode-go-local".to_string(),
        provider_type: "opencode_go".to_string(),
        display_name: "OpenCode Go".to_string(),
        auth_type: "local".to_string(),
        keychain_service: None,
        keychain_account: None,
    };
    refresh_provider_row(state.inner(), &row).await.map(|_| ())
}

#[tauri::command]
async fn add_codex_provider(state: State<'_, AppState>) -> Result<(), String> {
    let conn = open_db(&state.db_path)?;
    let now = now_iso();
    conn.execute(
        "INSERT OR IGNORE INTO providers
        (id, provider_type, display_name, auth_type, keychain_service, keychain_account, enabled, created_at, updated_at)
        VALUES (?1, 'codex', 'Codex', 'local', NULL, NULL, 1, ?2, ?2)",
        params!["codex-local", now],
    )
    .map_err(db_error)?;

    let row = ProviderRow {
        id: "codex-local".to_string(),
        provider_type: "codex".to_string(),
        display_name: "Codex".to_string(),
        auth_type: "local".to_string(),
        keychain_service: None,
        keychain_account: None,
    };
    refresh_provider_row(state.inner(), &row).await.map(|_| ())
}

#[tauri::command]
async fn add_openai_provider(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    add_api_key_provider(api_key, state.inner(), "openai", "OpenAI / GPT").await
}

#[tauri::command]
async fn add_openrouter_provider(
    api_key: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    add_api_key_provider(api_key, state.inner(), "openrouter", "OpenRouter").await
}

async fn add_api_key_provider(
    api_key: String,
    state: &AppState,
    provider_type: &str,
    display_name: &str,
) -> Result<(), String> {
    let api_key = api_key.trim().to_string();
    if api_key.is_empty() {
        return Err("API key is required.".to_string());
    }

    let provider_id = format!("{}-{}", provider_type, Uuid::new_v4());
    let service = keychain_service(provider_type);
    set_secret(&service, &provider_id, &api_key)?;

    let now = now_iso();
    {
        let conn = open_db(&state.db_path)?;
        conn.execute(
            "INSERT INTO providers
            (id, provider_type, display_name, auth_type, keychain_service, keychain_account, enabled, created_at, updated_at)
            VALUES (?1, ?2, ?3, 'api_key', ?4, ?1, 1, ?5, ?5)",
            params![provider_id, provider_type, display_name, service, now],
        )
        .map_err(db_error)?;
    }

    let row = ProviderRow {
        id: provider_id.clone(),
        provider_type: provider_type.to_string(),
        display_name: display_name.to_string(),
        auth_type: "api_key".to_string(),
        keychain_service: Some(keychain_service(provider_type)),
        keychain_account: Some(provider_id.clone()),
    };

    let snapshot = refresh_provider_row(state, &row).await;
    if let Err(message) = snapshot {
        let _ = delete_secret(&service, &provider_id);
        let conn = open_db(&state.db_path)?;
        let _ = conn.execute("DELETE FROM providers WHERE id = ?1", params![provider_id]);
        return Err(message);
    }

    Ok(())
}

#[tauri::command]
async fn refresh_all(state: State<'_, AppState>) -> Result<(), String> {
    refresh_all_internal(&state).await
}

#[tauri::command]
async fn refresh_provider(provider_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let row = {
        let conn = open_db(&state.db_path)?;
        load_provider(&conn, &provider_id)?.ok_or_else(|| "Provider not found.".to_string())?
    };
    refresh_provider_row(&state, &row).await.map(|_| ())
}

#[tauri::command]
fn remove_provider(provider_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let conn = open_db(&state.db_path)?;
    if let Some(row) = load_provider(&conn, &provider_id)? {
        if let (Some(service), Some(account)) = (row.keychain_service, row.keychain_account) {
            let _ = delete_secret(&service, &account);
        }
    }
    conn.execute(
        "DELETE FROM usage_snapshots WHERE provider_id = ?1",
        params![provider_id],
    )
    .map_err(db_error)?;
    conn.execute("DELETE FROM providers WHERE id = ?1", params![provider_id])
        .map_err(db_error)?;
    Ok(())
}

async fn refresh_all_internal(state: &AppState) -> Result<(), String> {
    let rows = {
        let conn = open_db(&state.db_path)?;
        load_providers(&conn)?
    };

    for row in rows {
        let _ = refresh_provider_row(state, &row).await;
    }
    Ok(())
}

async fn refresh_provider_row(
    state: &AppState,
    row: &ProviderRow,
) -> Result<UsageSnapshot, String> {
    let snapshot = match row.provider_type.as_str() {
        "mock" => Ok(mock_snapshot()),
        "codex" => refresh_codex(state, row).await,
        "opencode_go" => refresh_opencode_go().await,
        "openrouter" => refresh_openrouter(state, row).await,
        "openai" => refresh_openai(state, row).await,
        _ => Ok(UsageSnapshot {
            status: "unknown".to_string(),
            used: None,
            limit_value: None,
            remaining: None,
            percentage: None,
            unit: "unknown".to_string(),
            reset_at: None,
            message: Some("Provider adapter is not implemented yet.".to_string()),
            raw_json: None,
            created_at: now_iso(),
        }),
    };

    let snapshot = match snapshot {
        Ok(snapshot) => snapshot,
        Err(message) => UsageSnapshot {
            status: "error".to_string(),
            used: None,
            limit_value: None,
            remaining: None,
            percentage: None,
            unit: "unknown".to_string(),
            reset_at: None,
            message: Some(message.clone()),
            raw_json: None,
            created_at: now_iso(),
        },
    };

    let conn = open_db(&state.db_path)?;
    insert_snapshot(&conn, &row.id, snapshot.clone())?;

    if snapshot.status == "error" {
        Err(snapshot
            .message
            .clone()
            .unwrap_or_else(|| "Provider refresh failed.".to_string()))
    } else {
        Ok(snapshot)
    }
}

async fn refresh_opencode_go() -> Result<UsageSnapshot, String> {
    let (auth_path, db_path) = opencode_go_paths()?;
    let has_auth = opencode_go_has_auth(&auth_path);
    if !db_path.exists() {
        if has_auth {
            return Err(
                "OpenCode Go local database was not found at ~/.local/share/opencode/opencode.db."
                    .to_string(),
            );
        }
        return Err(
            "OpenCode Go was not detected. Sign in/use OpenCode Go locally first.".to_string(),
        );
    }

    let conn = Connection::open(&db_path).map_err(|err| {
        format!(
            "Could not open OpenCode Go local database at {}: {}",
            db_path.display(),
            err
        )
    })?;
    conn.busy_timeout(Duration::from_millis(250))
        .map_err(db_error)?;

    let rows = read_opencode_go_rows(&conn)?;
    if !has_auth && rows.is_empty() {
        return Err(
            "OpenCode Go was not detected. Sign in/use OpenCode Go locally first.".to_string(),
        );
    }
    if rows.is_empty() {
        return Err("OpenCode Go local usage history has no rows yet.".to_string());
    }

    let now = Utc::now();
    let now_ms = now.timestamp_millis();
    let five_hours_ms = 5 * 60 * 60 * 1000_i64;
    let week_ms = 7 * 24 * 60 * 60 * 1000_i64;
    let session_start_ms = now_ms - five_hours_ms;
    let weekday_offset = now.weekday().num_days_from_monday() as i64;
    let week_start_date = now.date_naive() - chrono::Duration::days(weekday_offset);
    let week_start_ms = week_start_date
        .and_hms_opt(0, 0, 0)
        .and_then(|date| Utc.from_local_datetime(&date).single())
        .map(|date| date.timestamp_millis())
        .unwrap_or(now_ms - week_ms);
    let week_end_ms = week_start_ms + week_ms;
    let month_start_ms = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .map(|date| date.timestamp_millis())
        .unwrap_or(now_ms);
    let (next_month_year, next_month) = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    let month_end_ms = Utc
        .with_ymd_and_hms(next_month_year, next_month, 1, 0, 0, 0)
        .single()
        .map(|date| date.timestamp_millis())
        .unwrap_or(month_start_ms + 30 * 24 * 60 * 60 * 1000_i64);

    let session_cost = sum_opencode_go_cost(&rows, session_start_ms, now_ms);
    let weekly_cost = sum_opencode_go_cost(&rows, week_start_ms, week_end_ms);
    let monthly_cost = sum_opencode_go_cost(&rows, month_start_ms, month_end_ms);
    let session_percent = percent(session_cost, 12.0);
    let weekly_percent = percent(weekly_cost, 30.0);
    let monthly_percent = percent(monthly_cost, 60.0);
    let reset_ms = rows
        .iter()
        .filter(|row| row.created_ms >= session_start_ms && row.created_ms < now_ms)
        .map(|row| row.created_ms)
        .min()
        .map(|oldest| oldest + five_hours_ms)
        .unwrap_or(now_ms);
    let reset_at = Utc
        .timestamp_millis_opt(reset_ms)
        .single()
        .map(|date| date.to_rfc3339());

    Ok(UsageSnapshot {
        status: status_from_percentage(Some(session_percent)),
        used: Some(session_percent),
        limit_value: Some(100.0),
        remaining: Some((100.0 - session_percent).max(0.0)),
        percentage: Some(session_percent),
        unit: "percentage".to_string(),
        reset_at,
        message: Some(format!(
            "Local OpenCode Go estimate. 5h: {:.1}% (${:.2}/$12). Weekly: {:.1}% (${:.2}/$30). Monthly: {:.1}% (${:.2}/$60).",
            session_percent, session_cost, weekly_percent, weekly_cost, monthly_percent, monthly_cost
        )),
        raw_json: Some(
            json!({
                "source": "local",
                "rows": rows.len(),
                "session_cost": session_cost,
                "weekly_cost": weekly_cost,
                "monthly_cost": monthly_cost,
                "session_percent": session_percent,
                "weekly_percent": weekly_percent,
                "monthly_percent": monthly_percent
            })
            .to_string(),
        ),
        created_at: now_iso(),
    })
}

#[derive(Debug)]
struct OpenCodeGoRow {
    created_ms: i64,
    cost: f64,
}

fn opencode_go_paths() -> Result<(PathBuf, PathBuf), String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not locate home directory for OpenCode Go.".to_string())?;
    let dir = home.join(".local").join("share").join("opencode");
    Ok((dir.join("auth.json"), dir.join("opencode.db")))
}

fn opencode_go_has_auth(path: &PathBuf) -> bool {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| {
            value
                .pointer("/opencode-go/key")
                .and_then(Value::as_str)
                .map(|key| !key.trim().is_empty())
        })
        .unwrap_or(false)
}

fn read_opencode_go_rows(conn: &Connection) -> Result<Vec<OpenCodeGoRow>, String> {
    let sql = if sqlite_has_table(conn, "part")? {
        "WITH message_costs AS (
          SELECT
            id AS messageID,
            CAST(COALESCE(json_extract(data, '$.time.created'), time_created) AS INTEGER) AS createdMs,
            CAST(json_extract(data, '$.cost') AS REAL) AS cost
          FROM message
          WHERE json_valid(data)
            AND json_extract(data, '$.providerID') = 'opencode-go'
            AND json_extract(data, '$.role') = 'assistant'
            AND json_type(data, '$.cost') IN ('integer', 'real')
        )
        SELECT createdMs, cost
        FROM message_costs
        UNION ALL
        SELECT
          CAST(COALESCE(json_extract(p.data, '$.time.created'), p.time_created, m.time_created) AS INTEGER) AS createdMs,
          CAST(json_extract(p.data, '$.cost') AS REAL) AS cost
        FROM part p
        JOIN message m ON m.id = p.message_id
        WHERE json_valid(p.data)
          AND json_valid(m.data)
          AND json_extract(p.data, '$.type') = 'step-finish'
          AND json_type(p.data, '$.cost') IN ('integer', 'real')
          AND json_extract(m.data, '$.providerID') = 'opencode-go'
          AND json_extract(m.data, '$.role') = 'assistant'
          AND NOT EXISTS (
            SELECT 1
            FROM message_costs
            WHERE message_costs.messageID = p.message_id
          )"
    } else {
        "SELECT
          CAST(COALESCE(json_extract(data, '$.time.created'), time_created) AS INTEGER) AS createdMs,
          CAST(json_extract(data, '$.cost') AS REAL) AS cost
        FROM message
        WHERE json_valid(data)
          AND json_extract(data, '$.providerID') = 'opencode-go'
          AND json_extract(data, '$.role') = 'assistant'
          AND json_type(data, '$.cost') IN ('integer', 'real')"
    };

    let mut stmt = conn.prepare(sql).map_err(db_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(OpenCodeGoRow {
                created_ms: row.get(0)?,
                cost: row.get(1)?,
            })
        })
        .map_err(db_error)?;

    rows.filter_map(|row| match row {
        Ok(row) if row.created_ms > 0 && row.cost.is_finite() && row.cost >= 0.0 => Some(Ok(row)),
        Ok(_) => None,
        Err(err) => Some(Err(db_error(err))),
    })
    .collect()
}

fn sqlite_has_table(conn: &Connection, table: &str) -> Result<bool, String> {
    conn.query_row(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
        params![table],
        |_| Ok(()),
    )
    .optional()
    .map(|value| value.is_some())
    .map_err(db_error)
}

fn sum_opencode_go_cost(rows: &[OpenCodeGoRow], start_ms: i64, end_ms: i64) -> f64 {
    rows.iter()
        .filter(|row| row.created_ms >= start_ms && row.created_ms < end_ms)
        .map(|row| row.cost)
        .sum()
}

fn percent(used: f64, limit: f64) -> f64 {
    if !used.is_finite() || limit <= 0.0 {
        return 0.0;
    }
    ((used / limit * 100.0).clamp(0.0, 100.0) * 10.0).round() / 10.0
}

async fn refresh_codex(state: &AppState, _row: &ProviderRow) -> Result<UsageSnapshot, String> {
    let auth_path = codex_auth_path()?;
    let mut auth_json: Value =
        serde_json::from_str(&fs::read_to_string(&auth_path).map_err(|_| {
            "Could not read ~/.codex/auth.json. Sign in to Codex first.".to_string()
        })?)
        .map_err(|_| "Codex auth.json is not valid JSON.".to_string())?;

    let access_token = auth_json
        .pointer("/tokens/access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "Codex auth.json does not contain an access token.".to_string())?
        .to_string();
    let account_id = auth_json
        .pointer("/tokens/account_id")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut response = codex_usage_request(state, &access_token, account_id.as_deref()).await?;
    if response.status().as_u16() == 401 {
        if let Some(new_access_token) = refresh_codex_token(state, &mut auth_json).await? {
            fs::write(
                &auth_path,
                serde_json::to_string_pretty(&auth_json)
                    .map_err(|_| "Could not serialize refreshed Codex auth.".to_string())?,
            )
            .map_err(|_| "Could not update ~/.codex/auth.json with refreshed token.".to_string())?;
            response = codex_usage_request(state, &new_access_token, account_id.as_deref()).await?;
        }
    }

    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("Codex session is expired. Please sign in to Codex again.".to_string());
    }
    if !status.is_success() {
        return Err(format!(
            "Codex usage endpoint returned HTTP {}.",
            status.as_u16()
        ));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|_| "Codex usage endpoint returned an unsupported response.".to_string())?;

    let primary = value.pointer("/rate_limit/primary_window");
    let percentage = primary
        .and_then(|window| window.get("used_percent"))
        .and_then(Value::as_f64);
    let reset_at = primary
        .and_then(|window| window.get("reset_at"))
        .and_then(Value::as_i64)
        .and_then(|seconds| Utc.timestamp_opt(seconds, 0).single())
        .map(|date| date.to_rfc3339());
    let secondary = value
        .pointer("/rate_limit/secondary_window/used_percent")
        .and_then(Value::as_f64);
    let plan = value
        .get("plan_type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let credits_balance = value.pointer("/credits/balance").and_then(Value::as_f64);
    let message = match (secondary, credits_balance) {
        (Some(weekly), Some(balance)) => format!(
            "Codex plan: {plan}. Weekly usage: {weekly:.0}%. Credits balance: {balance:.2}."
        ),
        (Some(weekly), None) => format!("Codex plan: {plan}. Weekly usage: {weekly:.0}%."),
        (None, Some(balance)) => format!("Codex plan: {plan}. Credits balance: {balance:.2}."),
        (None, None) => format!("Codex plan: {plan}."),
    };

    Ok(UsageSnapshot {
        status: status_from_percentage(percentage),
        used: percentage,
        limit_value: Some(100.0),
        remaining: percentage.map(|value| (100.0 - value).max(0.0)),
        percentage,
        unit: "percentage".to_string(),
        reset_at,
        message: Some(message),
        raw_json: Some(sanitize_raw(value)),
        created_at: now_iso(),
    })
}

async fn codex_usage_request(
    state: &AppState,
    access_token: &str,
    account_id: Option<&str>,
) -> Result<reqwest::Response, String> {
    let mut request = state
        .http
        .get("https://chatgpt.com/backend-api/wham/usage")
        .bearer_auth(access_token)
        .header("User-Agent", "codex-cli");
    if let Some(account_id) = account_id {
        request = request.header("ChatGPT-Account-Id", account_id);
    }
    request
        .send()
        .await
        .map_err(|_| "Could not reach Codex usage endpoint.".to_string())
}

async fn refresh_codex_token(
    state: &AppState,
    auth_json: &mut Value,
) -> Result<Option<String>, String> {
    let refresh_token = auth_json
        .pointer("/tokens/refresh_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "Codex session is expired and no refresh token is available.".to_string())?
        .to_string();

    let response = state
        .http
        .post("https://auth.openai.com/oauth/token")
        .json(&json!({
            "client_id": "app_EMoamEEZ73f0CkXaXp7hrann",
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "scope": "openid profile email"
        }))
        .send()
        .await
        .map_err(|_| "Could not refresh Codex session.".to_string())?;

    if !response.status().is_success() {
        return Err(
            "Codex refresh token is expired or revoked. Please sign in to Codex again.".to_string(),
        );
    }

    let value: Value = response
        .json()
        .await
        .map_err(|_| "Codex token refresh returned an unsupported response.".to_string())?;
    let access_token = value
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| "Codex token refresh did not return an access token.".to_string())?
        .to_string();

    if let Some(tokens) = auth_json.get_mut("tokens").and_then(Value::as_object_mut) {
        tokens.insert(
            "access_token".to_string(),
            Value::String(access_token.clone()),
        );
        if let Some(id_token) = value.get("id_token").and_then(Value::as_str) {
            tokens.insert("id_token".to_string(), Value::String(id_token.to_string()));
        }
        if let Some(refresh_token) = value.get("refresh_token").and_then(Value::as_str) {
            tokens.insert(
                "refresh_token".to_string(),
                Value::String(refresh_token.to_string()),
            );
        }
    }
    auth_json["last_refresh"] = Value::String(now_iso());
    Ok(Some(access_token))
}

fn codex_auth_path() -> Result<PathBuf, String> {
    if let Ok(home) = std::env::var("CODEX_HOME") {
        return Ok(PathBuf::from(home).join("auth.json"));
    }
    dirs::home_dir()
        .map(|home| home.join(".codex").join("auth.json"))
        .ok_or_else(|| "Could not locate home directory for Codex auth.".to_string())
}

async fn refresh_openai(state: &AppState, row: &ProviderRow) -> Result<UsageSnapshot, String> {
    let service = row
        .keychain_service
        .as_ref()
        .ok_or_else(|| "OpenAI credential pointer is missing.".to_string())?;
    let account = row
        .keychain_account
        .as_ref()
        .ok_or_else(|| "OpenAI credential account is missing.".to_string())?;
    let api_key = get_secret(service, account)?;

    let response = state
        .http
        .get("https://api.openai.com/v1/models")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| "Could not reach OpenAI. Try again later.".to_string())?;

    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("OpenAI API key is invalid or does not have access. Use an API key from platform.openai.com, not a ChatGPT Plus/Pro subscription.".to_string());
    }
    if status.as_u16() == 429 {
        return Err("OpenAI rate limit reached. Refresh later.".to_string());
    }
    if !status.is_success() {
        return Err(format!("OpenAI returned HTTP {}.", status.as_u16()));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|_| "OpenAI returned an unsupported response.".to_string())?;
    let model_count = value
        .get("data")
        .and_then(|data| data.as_array())
        .map(|items| items.len())
        .unwrap_or(0);

    Ok(UsageSnapshot {
        status: "unknown".to_string(),
        used: None,
        limit_value: None,
        remaining: None,
        percentage: None,
        unit: "usd".to_string(),
        reset_at: None,
        message: Some(format!(
            "OpenAI API key validated ({} models visible). API usage/cost data requires organization admin usage endpoints and is not the same as ChatGPT Plus/Pro quota.",
            model_count
        )),
        raw_json: None,
        created_at: now_iso(),
    })
}

async fn refresh_openrouter(state: &AppState, row: &ProviderRow) -> Result<UsageSnapshot, String> {
    let service = row
        .keychain_service
        .as_ref()
        .ok_or_else(|| "OpenRouter credential pointer is missing.".to_string())?;
    let account = row
        .keychain_account
        .as_ref()
        .ok_or_else(|| "OpenRouter credential account is missing.".to_string())?;
    let api_key = get_secret(service, account)?;

    let response = state
        .http
        .get("https://openrouter.ai/api/v1/credits")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| "Could not reach OpenRouter. Try again later.".to_string())?;

    let status = response.status();
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err("OpenRouter API key is invalid. Please update your credentials.".to_string());
    }
    if status.as_u16() == 429 {
        return Err("OpenRouter rate limit reached. Refresh later.".to_string());
    }
    if !status.is_success() {
        return Err(format!("OpenRouter returned HTTP {}.", status.as_u16()));
    }

    let value: Value = response
        .json()
        .await
        .map_err(|_| "OpenRouter returned an unsupported response.".to_string())?;
    let parsed: OpenRouterCreditsResponse = serde_json::from_value(value.clone())
        .map_err(|_| "OpenRouter returned an unsupported response.".to_string())?;

    let data = parsed
        .data
        .ok_or_else(|| "OpenRouter response did not include credit data.".to_string())?;
    let limit = data.total_credits;
    let used = data.total_usage;
    let remaining = match (limit, used) {
        (Some(limit), Some(used)) => Some((limit - used).max(0.0)),
        _ => None,
    };
    let percentage = match (used, limit) {
        (Some(used), Some(limit)) if limit > 0.0 => {
            Some(((used / limit) * 100.0).clamp(0.0, 100.0))
        }
        _ => None,
    };
    let status = status_from_percentage(percentage);
    let message = if percentage.is_some() {
        Some("Credit usage loaded from OpenRouter.".to_string())
    } else {
        Some("OpenRouter returned credit data, but no complete limit/usage pair.".to_string())
    };

    Ok(UsageSnapshot {
        status,
        used,
        limit_value: limit,
        remaining,
        percentage,
        unit: "usd".to_string(),
        reset_at: None,
        message,
        raw_json: Some(sanitize_raw(value)),
        created_at: now_iso(),
    })
}

fn init_db(path: &PathBuf) -> Result<(), String> {
    let conn = open_db(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS providers (
            id TEXT PRIMARY KEY,
            provider_type TEXT NOT NULL,
            display_name TEXT NOT NULL,
            auth_type TEXT NOT NULL,
            keychain_service TEXT NULL,
            keychain_account TEXT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS usage_snapshots (
            id TEXT PRIMARY KEY,
            provider_id TEXT NOT NULL,
            status TEXT NOT NULL,
            used REAL NULL,
            limit_value REAL NULL,
            remaining REAL NULL,
            percentage REAL NULL,
            unit TEXT NOT NULL,
            reset_at TEXT NULL,
            message TEXT NULL,
            raw_json TEXT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY(provider_id) REFERENCES providers(id)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );",
    )
    .map_err(db_error)?;
    Ok(())
}

fn open_db(path: &PathBuf) -> Result<Connection, String> {
    Connection::open(path).map_err(db_error)
}

fn load_providers(conn: &Connection) -> Result<Vec<ProviderRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, provider_type, display_name, auth_type, keychain_service, keychain_account
             FROM providers WHERE enabled = 1 ORDER BY created_at ASC",
        )
        .map_err(db_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProviderRow {
                id: row.get(0)?,
                provider_type: row.get(1)?,
                display_name: row.get(2)?,
                auth_type: row.get(3)?,
                keychain_service: row.get(4)?,
                keychain_account: row.get(5)?,
            })
        })
        .map_err(db_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(db_error)
}

fn load_provider(conn: &Connection, provider_id: &str) -> Result<Option<ProviderRow>, String> {
    conn.query_row(
        "SELECT id, provider_type, display_name, auth_type, keychain_service, keychain_account
         FROM providers WHERE id = ?1 AND enabled = 1",
        params![provider_id],
        |row| {
            Ok(ProviderRow {
                id: row.get(0)?,
                provider_type: row.get(1)?,
                display_name: row.get(2)?,
                auth_type: row.get(3)?,
                keychain_service: row.get(4)?,
                keychain_account: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(db_error)
}

fn load_provider_views(conn: &Connection) -> Result<Vec<ProviderView>, String> {
    let providers = load_providers(conn)?;
    providers
        .into_iter()
        .map(|provider| {
            let snapshot = latest_snapshot(conn, &provider.id)?;
            Ok(match snapshot {
                Some(snapshot) => ProviderView {
                    id: provider.id,
                    provider_type: provider.provider_type,
                    display_name: provider.display_name,
                    status: snapshot.status,
                    used: snapshot.used,
                    limit_value: snapshot.limit_value,
                    remaining: snapshot.remaining,
                    percentage: snapshot.percentage,
                    unit: snapshot.unit,
                    reset_at: snapshot.reset_at,
                    last_refresh_at: Some(snapshot.created_at),
                    message: snapshot.message,
                },
                None => ProviderView {
                    id: provider.id,
                    provider_type: provider.provider_type,
                    display_name: provider.display_name,
                    status: "unknown".to_string(),
                    used: None,
                    limit_value: None,
                    remaining: None,
                    percentage: None,
                    unit: "unknown".to_string(),
                    reset_at: None,
                    last_refresh_at: None,
                    message: Some(format!(
                        "{} is connected but has not been refreshed yet.",
                        provider.auth_type
                    )),
                },
            })
        })
        .collect()
}

fn latest_snapshot(conn: &Connection, provider_id: &str) -> Result<Option<UsageSnapshot>, String> {
    conn.query_row(
        "SELECT status, used, limit_value, remaining, percentage, unit, reset_at, message, raw_json, created_at
         FROM usage_snapshots WHERE provider_id = ?1 ORDER BY created_at DESC LIMIT 1",
        params![provider_id],
        |row| {
            Ok(UsageSnapshot {
                status: row.get(0)?,
                used: row.get(1)?,
                limit_value: row.get(2)?,
                remaining: row.get(3)?,
                percentage: row.get(4)?,
                unit: row.get(5)?,
                reset_at: row.get(6)?,
                message: row.get(7)?,
                raw_json: row.get(8)?,
                created_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(db_error)
}

fn insert_snapshot(
    conn: &Connection,
    provider_id: &str,
    snapshot: UsageSnapshot,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO usage_snapshots
        (id, provider_id, status, used, limit_value, remaining, percentage, unit, reset_at, message, raw_json, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            Uuid::new_v4().to_string(),
            provider_id,
            snapshot.status,
            snapshot.used,
            snapshot.limit_value,
            snapshot.remaining,
            snapshot.percentage,
            snapshot.unit,
            snapshot.reset_at,
            snapshot.message,
            snapshot.raw_json,
            snapshot.created_at,
        ],
    )
    .map_err(db_error)?;
    Ok(())
}

fn mock_snapshot() -> UsageSnapshot {
    let minute = Utc::now().timestamp() / 60;
    let percentage = match minute % 4 {
        0 => 32.0,
        1 => 76.0,
        2 => 94.0,
        _ => 58.0,
    };
    let limit = 1_000.0;
    let used = (limit * percentage) / 100.0;
    UsageSnapshot {
        status: status_from_percentage(Some(percentage)),
        used: Some(used),
        limit_value: Some(limit),
        remaining: Some(limit - used),
        percentage: Some(percentage),
        unit: "requests".to_string(),
        reset_at: Some((Utc::now() + chrono::Duration::hours(3)).to_rfc3339()),
        message: Some("Synthetic usage for UI validation.".to_string()),
        raw_json: None,
        created_at: now_iso(),
    }
}

fn status_from_percentage(percentage: Option<f64>) -> String {
    match percentage {
        Some(value) if value >= 90.0 => "critical".to_string(),
        Some(value) if value >= 70.0 => "warning".to_string(),
        Some(_) => "ok".to_string(),
        None => "unknown".to_string(),
    }
}

fn compute_global_status(providers: &[ProviderView]) -> String {
    if providers.is_empty() {
        return "unknown".to_string();
    }
    let statuses: Vec<&str> = providers
        .iter()
        .map(|provider| provider.status.as_str())
        .collect();
    if statuses.contains(&"error") {
        "error".to_string()
    } else if statuses.contains(&"critical") {
        "critical".to_string()
    } else if statuses.contains(&"warning") {
        "warning".to_string()
    } else if statuses.iter().all(|status| *status == "unknown") {
        "unknown".to_string()
    } else {
        "ok".to_string()
    }
}

fn update_tray_tooltip(app: &AppHandle, state: &AppState) {
    let tooltip = open_db(&state.db_path)
        .and_then(|conn| load_provider_views(&conn))
        .map(|providers| format!("QuotaTray — {}", compute_global_status(&providers)))
        .unwrap_or_else(|_| "QuotaTray — status unavailable".to_string());

    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn keychain_service(provider_type: &str) -> String {
    format!("{}.{}", KEYCHAIN_SERVICE_PREFIX, provider_type)
}

fn set_secret(service: &str, account: &str, secret: &str) -> Result<(), String> {
    Entry::new(service, account)
        .map_err(|_| "Could not access secure credential storage.".to_string())?
        .set_password(secret)
        .map_err(|_| "Could not save API key to secure credential storage.".to_string())
}

fn get_secret(service: &str, account: &str) -> Result<String, String> {
    Entry::new(service, account)
        .map_err(|_| "Could not access secure credential storage.".to_string())?
        .get_password()
        .map_err(|err| {
            format!(
                "Could not read API key from secure credential storage ({err}). Remove this provider and reconnect it."
            )
        })
}

fn delete_secret(service: &str, account: &str) -> Result<(), String> {
    Entry::new(service, account)
        .map_err(|_| "Could not access secure credential storage.".to_string())?
        .delete_credential()
        .map_err(|_| "Could not delete API key from secure credential storage.".to_string())
}

fn sanitize_raw(value: Value) -> String {
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn db_error(err: rusqlite::Error) -> String {
    format!("Local database error: {}", err)
}
