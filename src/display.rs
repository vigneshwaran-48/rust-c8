use std::io::Error;

use sdl2::{
    EventPump, Sdl, VideoSubsystem, pixels::Color, rect::Point, render::Canvas, video::Window,
};

pub struct Display {
    context: Sdl,
    video_system: VideoSubsystem,
    canvas: Canvas<Window>,
}

impl Display {
    pub fn init() -> Result<Self, String> {
        let context =
            sdl2::init().map_err(|e| format!("SDL context initialization failed: {}", e))?;
        let video_system = context
            .video()
            .map_err(|e| format!("Failed to initialize video system: {}", e))?;

        let window = video_system
            .window("Chip 8", 800, 600)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| format!("Failed to create window: {}", e))?;

        let mut canvas = window
            .into_canvas()
            .build()
            .map_err(|e| format!("Failed to create canvas: {}", e))?;

        // Set initial draw color and clear the screen
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        canvas.present();

        Ok(Self {
            context,
            video_system,
            canvas,
        })
    }

    pub fn event_pump(&self) -> Result<EventPump, String> {
        self.context.event_pump()
    }

    pub fn clear_screen(&mut self) -> Result<(), Error> {
        self.canvas.clear();
        Ok(())
    }

    pub fn draw_pixel(&mut self, x: i32, y: i32, color: Color) -> Result<(), String> {
        self.canvas.set_draw_color(color);
        self.canvas.draw_point(Point::new(x, y))?;
        Ok(())
    }
}
