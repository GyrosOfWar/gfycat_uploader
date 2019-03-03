extern crate reqwest;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate env_logger;
#[macro_use]
extern crate failure;
#[macro_use]
extern crate structopt;
extern crate azul;

mod cli;
mod error;
mod gui;
mod upload;

use error::Result;

fn main() -> Result<()> {
    gui::entry_point()
}
