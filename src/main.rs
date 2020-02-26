extern crate clap;
extern crate futures;
#[macro_use]
extern crate log;
extern crate simplelog;
#[macro_use]
extern crate lazy_static;

mod codecs;
mod util;

use clap::{App, SubCommand};
use codecs::{Codec, CodecError};
use std::io;
use std::process::exit;
pub use util::BitQueue;

fn io_wrapper<T: Codec>(mut codec: T) {
    match codec.stream(&mut io::stdin(), &mut io::stdout()) {
        Err(err) => error!("{:?}", err),
        _ => {}
    }
}

fn main() {
    simplelog::CombinedLogger::init(vec![simplelog::TermLogger::new(
        simplelog::LevelFilter::Info,
        simplelog::Config::default(),
    )
    .unwrap()])
    .unwrap();

    let matches = App::new("sigdig")
        .version("0.0.1")
        .author("R. Miles McCain <miles@rmrm.io>")
        .about("Basic digital signal processing tools")
        .subcommand(SubCommand::with_name("pipe").about("do nothing"))
        .subcommand(SubCommand::with_name("not").about("switch all bits"))
        .subcommand(SubCommand::with_name("huffenc").about("apply huffman encoding"))
        .subcommand(SubCommand::with_name("huffdec").about("apply huffman decoding"))
        .get_matches();

    match matches.subcommand() {
        ("pipe", Some(_)) => io_wrapper(codecs::Pipe::new()),
        ("not", Some(_)) => io_wrapper(codecs::Not::new()),
        ("huffenc", Some(_)) => io_wrapper(codecs::HuffmanEncoder::new(1048576)),
        ("huffdec", Some(_)) => io_wrapper(codecs::HuffmanDecoder::new()),
        _ => {
            error!("no valid command specified; try `--help`.");
            exit(1);
        }
    }
}
