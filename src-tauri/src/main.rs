// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::tray::TrayIconBuilder;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use std::io::Cursor;
use image::{RgbaImage, Rgba};
use imageproc::drawing::draw_text_mut;
use ab_glyph::{FontRef, PxScale};
use anyhow::{Context, Result};

fn generate_battery_icon(percentage: u32) -> Result<Image<'static>> {
    const SIZE: u32 = 64;

    let mut img = RgbaImage::new(SIZE, SIZE);
    let font = FontRef::try_from_slice(include_bytes!("../assets/arial.ttf"))
        .context("failed to load font")?;

    let scale = PxScale::from(SIZE as f32);
    let text = format!("{percentage}");
    draw_text_mut(&mut img, Rgba([0, 0, 0, 255]), 0, 0, scale, &font, &text);
    
    let mut icon_data: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    img.write_to(&mut icon_data, image::ImageFormat::Ico)
        .context("failed to write ICO data")?;
    
    let icon_img = Image::from_bytes(&icon_data.into_inner())
        .context("failed to create Tauri image from bytes")?
        .to_owned();

    Ok(icon_img)
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_i])?;
            let _tray = TrayIconBuilder::new()
                .icon(generate_battery_icon(42)?)
                .menu(&menu)
                .build(app)?;
            Ok(())
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
              println!("quit menu item was clicked");
              app.exit(0);
            }
            _ => {
              println!("menu item {:?} not handled", event.id);
            }
        })
        .run(tauri::generate_context!())
        .expect("error running Tauri app");
}