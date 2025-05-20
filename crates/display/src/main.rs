mod config;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::{Context, Result};
use exif::{In, Reader as ExifReader, Tag};
use image::{GenericImageView, imageops};
use notify::{
    RecommendedWatcher, RecursiveMode, Watcher,
    event::{CreateKind, EventKind, ModifyKind, RemoveKind},
};
use rand::seq::SliceRandom;
use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
};
use tokio::{
    sync::{Notify, watch},
    time::Instant,
};
use tracing_subscriber::EnvFilter;

use libs::{
    frame_settings::{FrameSettings, SharedSettings},
    util,
};

use config::CONFIG;

/// Collect all *.jpg / *.png files in a directory (non‑recursive).
fn scan_images(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("cannot read {:?}", dir))
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            matches!(
                p.extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .as_str(),
                "jpg" | "png"
            )
        })
        .collect();
    files.sort();
    files
}

/// Load an image and blit it full‑screen (keep aspect).
fn show_image(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    tex_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    img_path: &Path,
) -> Result<()> {
    let img_bytes = std::fs::read(img_path)?;
    let exif_orientation = ExifReader::new()
        .read_from_container(&mut std::io::Cursor::new(&img_bytes))
        .ok()
        .and_then(|exif| {
            exif.get_field(Tag::Orientation, In::PRIMARY)
                .and_then(|f| f.value.get_uint(0))
        })
        .unwrap_or(1);

    let mut dyn_img =
        image::load_from_memory(&img_bytes).with_context(|| format!("loading {img_path:?}"))?;

    // apply EXIF orientation
    dyn_img = match exif_orientation {
        2 => image::DynamicImage::ImageRgba8(imageops::flip_horizontal(&dyn_img)),
        3 => image::DynamicImage::ImageRgba8(imageops::rotate180(&dyn_img)),
        4 => image::DynamicImage::ImageRgba8(imageops::flip_vertical(&dyn_img)),
        5 => image::DynamicImage::ImageRgba8(imageops::rotate90(&imageops::flip_horizontal(
            &dyn_img,
        ))),
        6 => image::DynamicImage::ImageRgba8(imageops::rotate90(&dyn_img)),
        7 => image::DynamicImage::ImageRgba8(imageops::rotate270(&imageops::flip_horizontal(
            &dyn_img,
        ))),
        8 => image::DynamicImage::ImageRgba8(imageops::rotate270(&dyn_img)),
        _ => dyn_img,
    };

    // resize if needed
    let (w, h) = dyn_img.dimensions();
    let max_dimension = 2048;
    let scale = if w > max_dimension || h > max_dimension {
        let scale_w = max_dimension as f32 / w as f32;
        let scale_h = max_dimension as f32 / h as f32;
        scale_w.min(scale_h)
    } else {
        1.0
    };
    let scaled_w = (w as f32 * scale) as u32;
    let scaled_h = (h as f32 * scale) as u32;

    if scale < 1.0 {
        dyn_img = dyn_img.resize(scaled_w, scaled_h, image::imageops::FilterType::Lanczos3);
    }

    // convert to RGBA8 for SDL
    let rgba = dyn_img.into_rgba8();

    let pitch = scaled_w as usize * 4; // bytes per row

    let mut tex = tex_creator
        .create_texture_streaming(PixelFormatEnum::RGBA32, scaled_w, scaled_h)
        .context("create texture")?;
    tex.update(None, &rgba, pitch).unwrap();

    // scale to window while preserving aspect-ratio
    let (win_w, win_h) = canvas.output_size().unwrap();
    let scale = (win_w as f32 / scaled_w as f32).min(win_h as f32 / scaled_h as f32);
    let dst = Rect::from_center(
        (win_w as i32 / 2, win_h as i32 / 2),
        (scaled_w as f32 * scale) as u32,
        (scaled_h as f32 * scale) as u32,
    );

    canvas.clear();
    canvas.copy(&tex, None, dst).unwrap();
    canvas.present();
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("LOG_LEVEL"))
        .with_thread_ids(true)
        .init();

    let settings = SharedSettings::load(&CONFIG.backend_frame_settings_file)?;
    let settings_path = fs::canonicalize(&CONFIG.backend_frame_settings_file)
        .context("canonicalising BACKEND_FRAME_SETTINGS_FILE")?;

    let mut rx: watch::Receiver<FrameSettings> = settings.subscribe();
    let mut current = rx.borrow().clone();
    tracing::info!(?current, "initial settings");

    let data_dir = PathBuf::from(&CONFIG.backend_data_dir);
    let mut images = scan_images(&data_dir);
    if current.shuffle {
        images.shuffle(&mut rand::rng());
    }
    let mut index: usize = 0;
    tracing::info!(count = images.len(), "initial image scan");

    let (_watcher, mut watcher_rx) = {
        let (tx, rx) = tokio::sync::mpsc::channel::<notify::Result<notify::Event>>(8);
        let mut w: RecommendedWatcher = Watcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            notify::Config::default()
                .with_poll_interval(Duration::from_secs(5))
                .with_compare_contents(true),
        )?;
        w.watch(&data_dir, RecursiveMode::NonRecursive)?;
        let settings_dir = settings_path.parent().unwrap();
        w.watch(settings_dir, RecursiveMode::NonRecursive)?;
        (w, rx)
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut window = video_subsystem
        .window("Picture Frame", 800, 480)
        .position_centered()
        .fullscreen_desktop()
        .resizable()
        .build()
        .unwrap();
    window.set_bordered(false);

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .unwrap();
    let tex_creator = canvas.texture_creator();

    canvas.set_draw_color(Color::BLACK);
    canvas.clear();
    canvas.present();

    let shutdown = Arc::new(Notify::new());
    tokio::spawn(util::listen_for_shutdown(shutdown.clone()));

    let mut next_switch = Instant::now();

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,

            _ = rx.changed() => {
                current = rx.borrow().clone();
                tracing::debug!(?current, "settings updated via channel");
                next_switch = Instant::now();
                if current.shuffle {
                    images.shuffle(&mut rand::rng());
                    index = 0;
                }
            }

            Some(Ok(ev)) = watcher_rx.recv() => {
                tracing::debug!(?ev.paths, kind=?ev.kind, "fs event");

                let is_relevant = matches!(&ev.kind, EventKind::Modify(ModifyKind::Data(_)) |
                       EventKind::Modify(ModifyKind::Name(_)) |
                       EventKind::Create(CreateKind::File) |
                       EventKind::Create(CreateKind::Folder) |
                       EventKind::Remove(RemoveKind::File) |
                       EventKind::Remove(RemoveKind::Folder));

                let affects_settings = ev.paths.iter()
                    .any(|p| {
                        let canon = fs::canonicalize(p).ok();
                        canon.as_ref() == Some(&settings_path)
                            || p == &settings_path
                    });

                tracing::debug!(affects_settings, settings_path=?settings_path, "event affects settings?");

                if affects_settings {
                    if let Ok(toml) = fs::read_to_string(&settings_path) {
                        if let Ok(new) = toml::from_str::<FrameSettings>(&toml) {
                            // broadcast only if value changed
                            if new != *settings.settings_store.inner.read().await {
                                tracing::debug!(?new, "reloaded settings.toml");
                                *settings.settings_store.inner.write().await = new.clone();
                                let _ = settings.settings_store.tx.send(new.clone());
                                current = new;
                                next_switch = Instant::now();
                            }
                        } else {
                            tracing::warn!("TOML parse error");
                        }
                    } else {
                        tracing::warn!("Could not read settings file after event");
                    }
                } else if is_relevant {
                    images = scan_images(&data_dir);
                    tracing::debug!(count = images.len(), "image folder rescan");
                    if current.shuffle { images.shuffle(&mut rand::rng()); }
                    index = 0;
                    if images.is_empty() {
                        canvas.set_draw_color(Color::BLACK);
                        canvas.clear();
                        canvas.present();
                    }
                }
            }

            _ = tokio::time::sleep_until(next_switch), if current.display_enabled => {
                if !images.is_empty() {
                    let img = &images[index % images.len()];
                    tracing::debug!(
                        index = index,
                        total = images.len(),
                        rotate_enabled = current.rotate_enabled,
                        interval = current.rotate_interval_secs,
                        "showing next image"
                    );
                    if let Err(e) = show_image(&mut canvas, &tex_creator, img) {
                        tracing::error!("display error: {e:#}");
                    }
                    if current.rotate_enabled {
                        index = (index + 1) % images.len();
                    }
                } else {
                    canvas.set_draw_color(Color::BLACK);
                    canvas.clear();
                    canvas.present();
                }
                next_switch = Instant::now() + Duration::from_secs(current.rotate_interval_secs);
                tracing::debug!(
                    next_switch = ?next_switch,
                    "scheduled next switch"
                );
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(()),
                _ => {}
            }
        }

        if !current.display_enabled {
            canvas.set_draw_color(Color::BLACK);
            canvas.clear();
            canvas.present();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    Ok(())
}
