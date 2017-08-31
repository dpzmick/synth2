#![feature(specialization)]
#![feature(iterator_step_by)]
#![feature(cfg_target_feature)]
#![feature(test)]

#[cfg(test)]
extern crate test;

extern crate num;
extern crate serde;
extern crate simd;

#[macro_use]
extern crate enum_primitive;
extern crate easyjack as jack;

#[macro_use]
extern crate ketos;

#[macro_use]
extern crate ketos_derive;

pub mod audioprops;
pub mod components;
pub mod jack_engine;
pub mod midi;
pub mod patch;
pub mod ports;
pub mod soundscape;
pub mod topo;
pub mod util;
pub mod voice;
