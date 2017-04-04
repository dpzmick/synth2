#[macro_use] extern crate conrod;

use conrod::{widget, Colorable, Positionable, Widget};
use conrod::backend::glium::glium;
use conrod::backend::glium::glium::{DisplayBuild, Surface};
use conrod::event::Drag;

fn main() {
    const WIDTH: u32 = 400;
    const HEIGHT: u32 = 200;

    let display = glium::glutin::WindowBuilder::new()
        .with_vsync()
        .with_dimensions(WIDTH, HEIGHT)
        .with_title("Hello Conrod!")
        .build_glium()
        .unwrap();

    let mut ui = conrod::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();

    const FONT_PATH: &'static str = "/usr/share/fonts/TTF/Inconsolata-Regular.ttf";
    ui.fonts.insert_from_file(FONT_PATH).unwrap();

    let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();
    let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

    let mut last_update = std::time::Instant::now();
    let mut ui_needs_update = true;

    let mut st = ShittyState {
        pos1: [0.0, 0.0],
        id1: ui.widget_id_generator().next(),
        pos2: [0.0, -20.0],
        id2: ui.widget_id_generator().next(),
        id3: ui.widget_id_generator().next(),
    };

    'main: loop {
        let sixteen_ms = std::time::Duration::from_millis(16);
        let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
        if duration_since_last_update < sixteen_ms {
            std::thread::sleep(sixteen_ms - duration_since_last_update);
        }

        let mut events: Vec<_> = display.poll_events().collect();

        if events.is_empty() && !ui_needs_update {
            events.extend(display.wait_events().next());
        }

        ui_needs_update = false;


        last_update = std::time::Instant::now();

        // Handle all events.
        for event in events {

            // Use the `winit` backend feature to convert the winit event to a conrod one.
            if let Some(event) = conrod::backend::winit::convert(event.clone(), &display) {
                ui.handle_event(event);
                ui_needs_update = true;
            }

            match event {
                // Break from the loop upon `Escape`.
                glium::glutin::Event::KeyboardInput(_, _, Some(glium::glutin::VirtualKeyCode::Escape)) |
                    glium::glutin::Event::Closed =>
                    break 'main,
                _ => {},
            }
        }

        set_ui(ui.set_widgets(), &mut st);

        // Render the `Ui` and then display it on the screen.
        if let Some(primitives) = ui.draw_if_changed() {
            renderer.fill(&display, primitives, &image_map);
            let mut target = display.draw();
            target.clear_color(0.0, 0.0, 0.0, 1.0);
            renderer.draw(&display, &mut target, &image_map).unwrap();
            target.finish().unwrap();
        }
    }
}

use conrod::color::Color;
use conrod::position::Scalar;

// create a dragable rectangle
struct DragRect {
    common: widget::CommonBuilder,
    pub style: DragRectStyle,
}

widget_style! {
    style DragRectStyle {
        - color: Color { theme.shape_color }
        - border: Scalar { theme.border_width }
        - border_color: Color { theme.border_color }
    }
}

widget_ids! {
    pub struct DragRectIds {
        rectangle
    }
}

impl DragRect {
    fn new() -> Self {
        Self {
            common: widget::CommonBuilder::new(),
            style: DragRectStyle::new(),
        }
    }
}

impl Widget for DragRect {
    type State = DragRectIds;
    type Style = DragRectStyle;
    type Event = Option<conrod::event::Drag>;

    fn common(&self) -> &widget::CommonBuilder {
        &self.common
    }

    fn common_mut(&mut self) -> &mut widget::CommonBuilder {
        &mut self.common
    }

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State {
        DragRectIds::new(id_gen)
    }

    fn style(&self) -> Self::Style {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event {
        use conrod::widget::BorderedRectangle;
        use conrod::color;
        use conrod::Borderable;
        use conrod::input;
        use conrod::event;

        let widget::UpdateArgs { id, state, style, rect, ui, .. } = args;

        BorderedRectangle::new(rect.dim())
            .middle_of(id)
            .graphics_for(id)
            .border_color(color::LIGHT_RED)
            .set(state.rectangle, ui);

        let mut last_loc = None;
        for ev in ui.widget_input(id).events() {
            match ev {
                event::Widget::Drag(ev) if ev.button == input::MouseButton::Left => {
                    last_loc = Some(ev)
                },
                _ => ()
            }
        }

        last_loc
    }
}

struct ShittyState {
    pub pos1: conrod::Point,
    pub id1: conrod::widget::Id,

    pub pos2: conrod::Point,
    pub id2: conrod::widget::Id,

    pub id3: conrod::widget::Id,
}

fn set_ui(ref mut ui: conrod::UiCell, st: &mut ShittyState) {
    use conrod::Sizeable;
    use conrod::widget::Line;
    use conrod::widget::DropDownList;

    if let Some(loc) = DragRect::new()
                    .xy(st.pos1)
                    .wh([50.0, 50.0])
                    .set(st.id1, ui)
    {
        st.pos1[0] += loc.to[0];
        st.pos1[1] += loc.to[1];
    }

    if let Some(loc) = DragRect::new()
                    .xy(st.pos2)
                    .wh([50.0, 50.0])
                    .set(st.id2, ui)
    {
        st.pos2[0] += loc.to[0];
        st.pos2[1] += loc.to[1];
    }

    Line::new(st.pos1, st.pos2)
        .set(st.id3, ui);
}
