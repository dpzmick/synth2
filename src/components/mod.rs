mod traits;
pub use self::traits::*;

// list of all the components, kept in alphabetical order
mod combine;
mod math;
mod onoff;
mod sine;
mod square;

pub use self::combine::CombineInputs;
pub use self::math::Math;
pub use self::onoff::{OnOff, OnOffConfig};
pub use self::sine::{SineWaveOscillator, SineWaveOscillatorConfig};
pub use self::square::{SquareWaveOscillator, SquareWaveOscillatorConfig};

// macro_rules! all_components {
//     () => (
//         {
//             SineWaveOscillatorConfig,
//             SquareWaveOscillatorConfig
//         }
//     )
// }

// pub fn test() {
//     make_valid!( type all_components!{} );
// }
