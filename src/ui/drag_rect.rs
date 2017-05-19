use conrod;
use conrod::Colorable;
use conrod::Positionable;
use conrod::backend::glium::glium::{DisplayBuild, Surface};

/// Use this in this one place
pub struct HiddenState {
    loc: conrod::Point,
}

widget_style! {
    style Style {
        - color: conrod::Color { theme.shape_color }
        - border: conrod::Scalar { theme.border_width }
        - border_color: conrod::Color { theme.border_color }
    }
}

widget_ids! {
    pub struct Ids {
        rectangle,
        text
    }
}

/// A DragRect moves itself as it is dragged around
pub struct DragRect<'a> {
    common: conrod::widget::CommonBuilder,
    name: String,
    pub style: Style,
    state: &'a mut HiddenState,
}

impl<'a> DragRect<'a> {
    pub fn new(name: String, state: &'a mut HiddenState) -> Self
    {
        Self {
            common: conrod::widget::CommonBuilder::new(),
            style: Style::new(),
            name,
            state,
        }
    }

    pub fn make_state(initial_position: conrod::Point) -> HiddenState
    {
        HiddenState { loc: initial_position }
    }
}

impl<'a> conrod::Widget for DragRect<'a> {
    // can't use this for the state: We need to be able to access the state and the
    // elements of the
    // widget struct in some key places
    type State = Ids;
    type Style = Style;
    type Event = Option<conrod::event::Drag>;

    fn common(&self) -> &conrod::widget::CommonBuilder
    {
        &self.common
    }

    fn default_x_position(&self, _: &conrod::Ui) -> conrod::Position
    {
        conrod::Position::Absolute(self.state.loc[0])
    }

    fn default_y_position(&self, _: &conrod::Ui) -> conrod::Position
    {
        conrod::Position::Absolute(self.state.loc[1])
    }

    fn common_mut(&mut self) -> &mut conrod::widget::CommonBuilder
    {
        &mut self.common
    }

    fn init_state(&self, id_gen: conrod::widget::id::Generator) -> Self::State
    {
        Ids::new(id_gen)
    }

    fn style(&self) -> Self::Style
    {
        self.style.clone()
    }

    fn update(mut self, args: conrod::widget::UpdateArgs<Self>) -> Self::Event
    {
        use conrod::widget::BorderedRectangle;
        use conrod::color;
        use conrod::Borderable;
        use conrod::input;
        use conrod::event;

        let shared_state: &mut HiddenState = self.state;
        let conrod::widget::UpdateArgs {
            id,
            style,
            rect,
            state,
            ui,
            ..
        } = args;

        BorderedRectangle::new(rect.dim())
            .middle_of(id)
            .graphics_for(id)
            .border_color(color::LIGHT_RED)
            .set(state.rectangle, ui);

        conrod::widget::Text::new(&self.name)
            .middle_of(state.rectangle)
            .color(color::BLACK)
            .set(state.text, ui);

        let mut last_ev = None;
        for ev in ui.widget_input(id).events() {
            match ev {
                event::Widget::Drag(ev) if ev.button == input::MouseButton::Left => {
                    // last_ev = Some(DragRectEv::Drag(ev))
                    last_ev = Some(ev)
                },

                // event::Widget::Click(ev) if ev.button == input::MouseButton::Right => {
                //     last_ev = Some(DragRectEv::RightClick(ev))
                // },
                _ => (),
            }
        }

        // the element jumps on the first click. The drag appears to set initial
        // position without
        // actually animating the blob

        // if the rect was dragged, update its current known state with the new drag
        // location
        match last_ev {
            Some(drag) => {
                println!("dragged, state is now: {:?} drag {:?}",
                         shared_state.loc,
                         drag);
                shared_state.loc[0] += drag.to[0];
                shared_state.loc[1] += drag.to[1];
            },
            None => (),
        }

        last_ev
    }
}
