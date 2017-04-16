use conrod;
use conrod::backend::glium::glium;
use conrod::backend::glium::glium::DisplayBuild;
use conrod::backend::glium::glium::Surface;
use conrod::glium::backend::glutin_backend::GlutinFacade;
use conrod::glium::glutin::Event;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 200;
const FONT_PATH: &'static str = "/usr/share/fonts/TTF/Inconsolata-Regular.ttf";

#[derive(Debug)]
pub enum UiEvent {
    Exit,
}

pub struct SynthUi {
    display: GlutinFacade,
    ui: conrod::Ui,
    renderer: conrod::backend::glium::Renderer,
    image_map: conrod::image::Map<glium::texture::Texture2d>,
}


impl SynthUi {
    /// Initializes a UI window
    pub fn new() -> Self {
        let display = glium::glutin::WindowBuilder::new()
            .with_vsync()
            .with_dimensions(WIDTH, HEIGHT)
            .with_title("Hello Conrod!")
            .build_glium()
            .unwrap();

        let mut ui = conrod::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();
        ui.fonts.insert_from_file(FONT_PATH).unwrap();

        let renderer = conrod::backend::glium::Renderer::new(&display).unwrap();
        let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

        SynthUi {
            display,
            ui,
            renderer,
            image_map,
        }
    }

    /// Update the UI and
    pub fn event_loop(&mut self) -> Vec<UiEvent> {
        let mut events: Vec<_> = self.display.poll_events().collect();

        if events.is_empty() { // && !ui_needs_update {
            events.extend(self.display.wait_events().next());
        }

        //ui_needs_update = false;

        // Handle all events.
        for event in events {
            // Use the `winit` backend feature to convert the winit event to a conrod one.
            if let Some(event) = conrod::backend::winit::convert(event.clone(), &self.display) {
                self.ui.handle_event(event);
                //ui_needs_update = true;
            }

            match event {
                glium::glutin::Event::Closed => return vec![UiEvent::Exit],
                _ => {},
            }
        }

        //set_ui(ui.set_widgets(), &mut st);

        // Render the `Ui` and then display it on the screen.
        if let Some(primitives) = self.ui.draw_if_changed() {
            self.renderer.fill(&self.display, primitives, &self.image_map);
            let mut target = self.display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            self.renderer.draw(&self.display, &mut target, &self.image_map).unwrap();
            target.finish().unwrap();
        }

        Vec::new()
    }
}
