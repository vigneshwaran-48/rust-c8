use std::io::Error;

use sdl2::{
    EventPump, Sdl, VideoSubsystem,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::{Canvas, TextureCreator},
    video::{Window, WindowContext},
};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

pub struct Display {
    context: Sdl,
    video_system: VideoSubsystem,
    canvas: Canvas<Window>,
    texture_creator: TextureCreator<WindowContext>,
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

        let texture_creator = canvas.texture_creator();

        Ok(Self {
            context,
            video_system,
            canvas,
            texture_creator,
        })
    }

    pub fn event_pump(&self) -> Result<EventPump, String> {
        self.context.event_pump()
    }

    pub fn clear_screen(&mut self) -> Result<(), Error> {
        self.canvas.clear();
        Ok(())
    }

    pub fn draw(&mut self, screen: &[u8]) -> Result<(), String> {
        let mut texture = self
            .texture_creator
            .create_texture_streaming(PixelFormatEnum::RGB24, WIDTH as u32, HEIGHT as u32)
            .unwrap();
        texture
            .with_lock(None, |buffer: &mut [u8], _pitch| {
                for (i, &pixel) in screen.iter().enumerate() {
                    let color = if pixel == 1 { 255 } else { 0 };
                    let offset = i * 3;
                    buffer[offset] = color;
                    buffer[offset + 1] = color;
                    buffer[offset + 2] = color;
                }
            })
            .unwrap();

        self.canvas.copy(&texture, None, None).unwrap();
        self.canvas.present();
        Ok(())
    }
}
