#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod battery_icon_generator;
use battery_icon_generator::BatteryIconGenerator;

use std::{sync::Arc, thread, time::Duration};
use anyhow::{Context, Result};
use battery::{Manager, State};
use tauri::{
    async_runtime, menu::{Menu, MenuItem}, tray::{TrayIcon, TrayIconBuilder}, App, AppHandle, Wry
};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;

/// 在独立线程中定期读取电池电量并发送消息
fn spawn_battery_monitor(tx: Sender<(u32, State)>) {
    thread::spawn(move || {
        let manager = Manager::new().expect("Failed to initialize battery manager");
        let mut last_battery_info: Option<(u32, State)> = None;

        loop {
            if let Ok(batteries) = manager.batteries() {
                if let Some(battery) = batteries.flatten().next() {
                    let percentage = (battery.state_of_charge().value * 100.0).round() as u32;
                    let state = battery.state();

                    if Some((percentage, state)) != last_battery_info {
                        last_battery_info = Some((percentage, state));

                        tx.blocking_send((percentage, state))
                            .expect("Failed to send battery info");
                    }
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });
}

fn init_menu(app: &AppHandle) -> Result<Menu<Wry>, tauri::Error> {
    let autostart_status = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(false);
    let autostart_item = MenuItem::with_id(
        app,
        "autostart",
        format!("{} autostart", if autostart_status { "Disable" } else { "Enable" }),
        true,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(
        app, 
        "quit", 
        "Quit", 
        true, 
        None::<&str>
    )?;
    Menu::with_items(app, &[&autostart_item, &quit_item])
}

/// 初始化托盘图标和菜单
fn init_tray(app: &mut App) -> Result<()> {
    let tray_icon = TrayIconBuilder::with_id("tray_id")
        .menu(&init_menu(app.handle())?)
        .build(app)?;
    let tray = Arc::new(Mutex::new(tray_icon));

    let (tx, rx) = channel(1);
    spawn_battery_monitor(tx);
    spawn_tray_updater(tray, rx);

    Ok(())
}

/// 启动异步任务监听电池更新并修改托盘图标
fn spawn_tray_updater(tray: Arc<Mutex<TrayIcon>>, mut rx: Receiver<(u32, State)>) {
    async_runtime::spawn(async move {
        let icon_generator = BatteryIconGenerator::new().unwrap();
        while let Some((percentage, state)) = rx.recv().await {
            if let Ok(icon) = icon_generator.generate_icon(percentage, state == State::Charging).await {
                let tooltip = match state {
                    State::Charging => format!("Charging: {}%", percentage),
                    State::Discharging => format!("Discharging: {}%", percentage),
                    State::Full => format!("Full"),
                    _ => format!("Unhandled state: {}%", percentage),
                };

                let tray = tray.lock().await;
                if let Err(e) = tray.set_icon(Some(icon)) {
                    eprintln!("Failed to update tray icon: {}", e);
                }
                if let Err(e) = tray.set_tooltip(Some(&tooltip)) {
                    eprintln!("Failed to update tray tooltip: {}", e);
                }
            }
        }
    });
}

#[tokio::main]
async fn main() {
    tauri::Builder::default()
        .setup(|app| {
            app.handle().plugin(tauri_plugin_autostart::init(
                MacosLauncher::LaunchAgent,
                None,
            )).context("Error initializing autostart plugin")?;

            init_tray(app)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                println!("User clicked quit");
                app.exit(0);
            }
            "autostart" => {
                let autostart = app.autolaunch();
                let enabled = autostart.is_enabled().unwrap_or(false);
                if enabled {
                    let _ = autostart.disable();
                } else {
                    let _ = autostart.enable();
                }
                let tray = app.tray_by_id("tray_id").expect("Failed to get tray handle");
                tray.set_menu(init_menu(app).ok()).expect("Failed to update tray menu");
            }
            other => {
                println!("Unhandled menu item: {:?}", other);
            }
        })
        .run(tauri::generate_context!())
        .expect("Error running Tauri app");
}
