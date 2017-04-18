
use conrod;

use conrod::Positionable;
use conrod::Sizeable;
use conrod::Widget;
use conrod::backend::glium::glium;
use conrod::backend::glium::glium::DisplayBuild;
use conrod::backend::glium::glium::Surface;
use conrod::glium::backend::glutin_backend::GlutinFacade;
use conrod::glium::glutin::Event;
use conrod::widget::Line;

use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::collections::HashSet;
use ui::drag_rect::DragRect;

use voice::Voice;

const WIDTH: u32 = 3000;
const HEIGHT: u32 = 2000;
const FONT_PATH: &'static str = "/usr/share/fonts/TTF/Inconsolata-Regular.ttf";

#[derive(Debug)]
pub enum UiEvent {
    Exit,
}

// TODO rename this, this is some component?
// this exists to hide most of the conrod boilerplate and focus only on the logic that we actually
// care about
struct LogicalUi<'a> {
    voice: Voice<'a>,
    ids: HashMap<String, conrod::widget::Id>,
    rect_loc: HashMap<String, conrod::Point>,
}

impl<'a> LogicalUi<'a> {
    fn new() -> Self
    {
        let mut voice = Voice::new();
        voice.example_connections();

        Self {
            voice,
            ids: HashMap::new(),
            rect_loc: HashMap::new(),
        }
    }

    /// Get or create an id for an element
    fn get_id(&mut self, name: String, ui: &mut conrod::UiCell) -> conrod::widget::Id
    {
        *self.ids
             .entry(name)
             .or_insert_with(|| ui.widget_id_generator().next())
    }

    fn draw(&mut self, ui: &mut conrod::UiCell)
    {
        let mut i = 0.0;
        for comp in self.voice.get_components() {
            self.rect_loc
                .entry(comp.clone())
                .or_insert([i * 20.0, i * 20.0]);
            let id = self.get_id(comp.clone(), ui);

            if let Some(loc) = DragRect::new(comp.clone())
                   .xy(self.rect_loc[&comp])
                   .wh([200.0, 50.0])
                   .set(id, ui)
            {
                let mut rect_loc = self.rect_loc.get_mut(&comp.clone()).unwrap();
                rect_loc[0] += loc.to[0];
                rect_loc[1] += loc.to[1];
            }

            i += 1.0;
        }

        // for all connections, draw a line
        let mut all_conns = HashSet::new();
        for (c1, c2) in self.voice.get_connections().into_iter() {
            if c1 < c2 {
                all_conns.insert( (c1.to_string(), c2.to_string()) );
            } else {
                all_conns.insert( (c2.to_string(), c1.to_string()) );
            }
        }

        for (c1, c2) in all_conns.into_iter() {
            let nid = c1.clone() + ":" + &c2;
            let id = self.get_id(nid, ui);
            Line::new(self.rect_loc[&c1], self.rect_loc[&c2])
                .thickness(5.0)
                .set(id, ui);
        }
    }
}

pub struct SynthUi<'a> {
    display: GlutinFacade,
    ui: conrod::Ui,
    renderer: conrod::backend::glium::Renderer,
    image_map: conrod::image::Map<glium::texture::Texture2d>,
    ui_needs_update: bool,
    logic: LogicalUi<'a>,
}

// private impl
impl<'a> SynthUi<'a> {
    fn draw_ui(&mut self)
    {
        self.logic.draw(&mut self.ui.set_widgets());
    }
}

impl<'a> SynthUi<'a> {
    /// Initializes a UI window
    pub fn new() -> Self
    {
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
            ui_needs_update: false,
            logic: LogicalUi::new(),
        }
    }

    /// Update the UI and
    pub fn event_loop(&mut self) -> Vec<UiEvent>
    {
        let mut events: Vec<_> = self.display.poll_events().collect();

        if events.is_empty() && !self.ui_needs_update {
            events.extend(self.display.wait_events().next());
        }

        self.ui_needs_update = false;

        // Handle all events.
        for event in events {
            // Use the `winit` backend feature to convert the winit event to a conrod one.
            if let Some(event) = conrod::backend::winit::convert(event.clone(), &self.display) {
                println!("{:?}", event);
                self.ui.handle_event(event);
                self.ui_needs_update = true;
            }

            match event {
                glium::glutin::Event::Closed => return vec![UiEvent::Exit],
                _ => {},
            }
        }

        self.draw_ui();

        // Render the `Ui` and then display it on the screen.
        if let Some(primitives) = self.ui.draw_if_changed() {
            self.renderer
                .fill(&self.display, primitives, &self.image_map);
            let mut target = self.display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            self.renderer
                .draw(&self.display, &mut target, &self.image_map)
                .unwrap();
            target.finish().unwrap();
        }

        Vec::new()
    }
}
