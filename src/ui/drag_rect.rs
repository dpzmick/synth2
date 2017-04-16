
use conrod;
use conrod::{Colorable, Positionable, Widget, widget};
use conrod::Color;
use conrod::Scalar;
use conrod::backend::glium::glium;
use conrod::backend::glium::glium::{DisplayBuild, Surface};
use conrod::event::Drag;

widget_style! {
    style DragRectStyle {
        - color: Color { theme.shape_color }
        - border: Scalar { theme.border_width }
        - border_color: Color { theme.border_color }
    }
}

widget_ids! {
    pub struct DragRectIds {
        rectangle,
        text
    }
}

pub struct DragRect {
    common: conrod::widget::CommonBuilder,
    name: String,
    pub style: DragRectStyle,
}


impl DragRect {
    pub fn new(name: String) -> Self
    {
        Self {
            common: widget::CommonBuilder::new(),
            style: DragRectStyle::new(),
            name,
        }
    }
}

// enum DragRectEv {
//     Drag(conrod::event::Drag),
//     RightClick(conrod::event::Click),
// }

impl Widget for DragRect {
    type State = DragRectIds;
    type Style = DragRectStyle;
    type Event = Option<Drag>;

    fn common(&self) -> &widget::CommonBuilder
    {
        &self.common
    }

    fn common_mut(&mut self) -> &mut widget::CommonBuilder
    {
        &mut self.common
    }

    fn init_state(&self, id_gen: widget::id::Generator) -> Self::State
    {
        DragRectIds::new(id_gen)
    }

    fn style(&self) -> Self::Style
    {
        self.style.clone()
    }

    fn update(self, args: widget::UpdateArgs<Self>) -> Self::Event
    {
        use conrod::widget::BorderedRectangle;
        use conrod::color;
        use conrod::Borderable;
        use conrod::input;
        use conrod::event;

        let widget::UpdateArgs {
            id,
            state,
            style,
            rect,
            ui,
            ..
        } = args;

        BorderedRectangle::new(rect.dim())
            .middle_of(id)
            .graphics_for(id)
            .border_color(color::LIGHT_RED)
            .set(state.rectangle, ui);

        widget::Text::new(&self.name)
            .middle_of(state.rectangle)
            .color(color::BLACK)
            .set(state.text, ui);

        let mut last_ev = None;
        for ev in ui.widget_input(id).events() {
            match ev {
                event::Widget::Drag(ev) if ev.button == input::MouseButton::Left => {
                    //last_ev = Some(DragRectEv::Drag(ev))
                    last_ev = Some(ev)
                },

                // event::Widget::Click(ev) if ev.button == input::MouseButton::Right => {
                //     last_ev = Some(DragRectEv::RightClick(ev))
                // },
                _ => (),
            }
        }

        last_ev
    }
}
