mod config;

use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result};
use image::GenericImageView;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use rand::seq::SliceRandom;
use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, rect::Rect};
use tokio::{sync::watch, time::Instant};

use libs::frame_settings::{FrameSettings, SharedSettings};

use config::CONFIG;

// Gather all *.jpg *.png under the given dir.
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

/// Load an image and blit it to the full-screen canvas (keeping aspect).
fn show_image(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    tex_creator: &sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    img_path: &Path,
) -> Result<()> {
    let dyn_img = image::open(img_path).with_context(|| format!("loading {img_path:?}"))?;
    let (w, h) = dyn_img.dimensions();
    let rgba = dyn_img.into_rgba8();
    let pitch = w as usize * 4; // bytes per row

    let mut tex = tex_creator
        .create_texture_streaming(PixelFormatEnum::RGBA32, w, h)
        .context("create texture")?;

    // copy the whole buffer in one call
    tex.update(None, &rgba, pitch).unwrap();

    // scale to window  while preserving aspect-ratio
    let (win_w, win_h) = canvas.output_size().unwrap();
    let scale = (win_w as f32 / w as f32).min(win_h as f32 / h as f32);
    let dst = Rect::from_center(
        (win_w as i32 / 2, win_h as i32 / 2),
        (w as f32 * scale) as u32,
        (h as f32 * scale) as u32,
    );

    canvas.clear();
    canvas.copy(&tex, None, dst).unwrap();
    canvas.present();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let settings = SharedSettings::load(&CONFIG.backend_frame_settings_file)?;
    let mut rx: watch::Receiver<FrameSettings> = settings.subscribe();
    let mut current = rx.borrow().clone();

    let data_dir = PathBuf::from(&CONFIG.backend_data_dir);
    let mut images = scan_images(&data_dir);
    if current.shuffle {
        images.shuffle(&mut rand::rng())
    }
    let mut index: usize = 0;

    // BUG: This does not react, on settings change.
    let (mut _inotify, mut watcher_rx) = {
        let (tx, rx) = tokio::sync::mpsc::channel(8);
        let watcher_config = notify::Config::default()
            .with_poll_interval(Duration::from_secs(2))
            .with_compare_contents(true);
        let mut w: RecommendedWatcher = Watcher::new(
            move |res| {
                let _ = tx.blocking_send(res);
            },
            watcher_config,
        )
        .unwrap();
        w.watch(&data_dir, RecursiveMode::NonRecursive)?;
        (w, rx)
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut window = video_subsystem
        .window("Test", 800, 480)
        .position_centered()
        .fullscreen_desktop()
        .build()
        .unwrap();
    window.set_bordered(false);

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let tex_creator = canvas.texture_creator();

    let mut next_switch = Instant::now();
    let mut event_pump = sdl_context.event_pump().unwrap();

    loop {
        tokio::select! {
            _ = rx.changed() => {
                current = rx.borrow().clone();
                next_switch = Instant::now();
                if current.shuffle {
                    images.shuffle(&mut rand::rng());
                    index = 0;
                }
            }

            Some(_) = watcher_rx.recv() => {
                images = scan_images(&data_dir);
                if current.shuffle {
                    images.shuffle(&mut rand::rng());
                }
                index = 0.min(index);
            }

            _ = tokio::time::sleep_until(next_switch), if current.display_enabled => {
                if !images.is_empty() {
                    let img = &images[index % images.len()];
                    if let Err(e) = show_image(&mut canvas, &tex_creator, img) {
                        eprintln!("display error: {e:#}");
                    }
                    if current.rotate_enabled {
                        index = (index + 1) % images.len();
                    }
                }
                next_switch = Instant::now() + Duration::from_secs(current.rotate_interval_secs);
            }
        }

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return Ok(()),
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return Ok(()),
                Event::MouseButtonDown { .. } => {
                    // BUG: this freezes the screen
                    // next_switch = Instant::now();
                }
                _ => {}
            }
        }

        if !current.display_enabled {
            // NOTE: blank screen
            canvas.set_draw_color(sdl2::pixels::Color::BLACK);
            canvas.clear();
            canvas.present();
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }
}
