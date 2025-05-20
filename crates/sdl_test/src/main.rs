use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::Color,
};

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let mut window = video_subsystem
        .window("Picture Frame", 800, 480)
        .position_centered()
        .fullscreen_desktop()
        .build()
        .unwrap();
    window.set_bordered(false);

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let _tex_creator = canvas.texture_creator();

    let display_enabled = true;

    loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => return,
                _ => {}
            }
        }

        if display_enabled {
            canvas.set_draw_color(Color::BLACK);
            canvas.clear();
            canvas.present();
        }
    }
}
