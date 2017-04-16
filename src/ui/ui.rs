use std::collections::HashMap;

use conrod;
use conrod::backend::glium::glium;
use conrod::backend::glium::glium::DisplayBuild;
use conrod::backend::glium::glium::Surface;
use conrod::glium::backend::glutin_backend::GlutinFacade;
use conrod::glium::glutin::Event;
use conrod::Positionable;
use conrod::Sizeable;
use conrod::Widget;

use voice::Voice;
use ui::drag_rect::DragRect;

use std::cell::UnsafeCell;

const WIDTH: u32 = 400;
const HEIGHT: u32 = 200;
const FONT_PATH: &'static str = "/usr/share/fonts/TTF/Inconsolata-Regular.ttf";

#[derive(Debug)]
pub enum UiEvent {
    Exit,
}

pub struct SynthUi<'a> {
    display: GlutinFacade,
    ui: UnsafeCell<conrod::Ui>,
    renderer: conrod::backend::glium::Renderer,
    image_map: conrod::image::Map<glium::texture::Texture2d>,
    ui_needs_update: bool,

    // actual synth logic
    voice: Voice<'a>,
    ids: HashMap<String, conrod::widget::Id>,
    rect_loc: HashMap<String, conrod::Point>,
}

// private impl
impl<'a> SynthUi<'a> {
    /// Get or create an id for an element
    fn get_id(&mut self, name: String) -> conrod::widget::Id {
        let mut gen = unsafe { (*self.ui.get()).widget_id_generator() };
        *self.ids.entry(name).or_insert_with(|| gen.next())
    }

    fn draw_ui(&mut self) {
        let mut cell = unsafe { (*self.ui.get()).set_widgets() };

        let mut i = 0.0;
        for comp in self.voice.get_components() {
            self.rect_loc.entry(comp.clone()).or_insert([i * 20.0, i * 20.0]);
            let id = self.get_id(comp.clone());
            println!("comp: {}, loc: {:?}, id: {:?}", comp, self.rect_loc[&comp], id);

            if let Some(loc) = DragRect::new(comp.clone())
                                .xy(self.rect_loc[&comp.clone()])
                                .wh([50.0, 50.0])
                                .set(id, &mut cell)
            {
                println!("got event: {:?}", loc);
                let mut rect_loc = self.rect_loc.get_mut(&comp.clone()).unwrap();
                rect_loc[0] += loc.to[0];
                rect_loc[1] += loc.to[1];
            }

            i += 1.0;
        }
    }
}

impl<'a> SynthUi<'a> {
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

        let mut voice = Voice::new();
        voice.example_connections();

        SynthUi {
            display,
            ui: UnsafeCell::new(ui),
            renderer,
            image_map,
            ui_needs_update: false,
            voice,
            ids: HashMap::new(),
            rect_loc: HashMap::new(),
        }
    }

    /// Update the UI and
    pub fn event_loop(&mut self) -> Vec<UiEvent> {
        let mut events: Vec<_> = self.display.poll_events().collect();

        if events.is_empty() && !self.ui_needs_update {
            events.extend(self.display.wait_events().next());
        }

        self.ui_needs_update = false;

        // Handle all events.
        for event in events {
            // Use the `winit` backend feature to convert the winit event to a conrod one.
            if let Some(event) = conrod::backend::winit::convert(event.clone(), &self.display) {
                unsafe { (*self.ui.get()).handle_event(event) };
                self.ui_needs_update = true;
            }

            match event {
                glium::glutin::Event::Closed => return vec![UiEvent::Exit],
                _ => {},
            }
        }

        self.draw_ui();

        // Render the `Ui` and then display it on the screen.
        if let Some(primitives) = unsafe { (*self.ui.get()).draw_if_changed() } {
            self.renderer.fill(&self.display, primitives, &self.image_map);
            let mut target = self.display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            self.renderer.draw(&self.display, &mut target, &self.image_map).unwrap();
            target.finish().unwrap();
        }

        Vec::new()
    }
}
