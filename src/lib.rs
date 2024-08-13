#![feature(pin_deref_mut)]
#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(impl_trait_in_assoc_type)]

use std::io::{self, ErrorKind, SeekFrom};

use buf::DataReadBuf;

pub mod buf;
pub mod or;

pub mod reader;
pub mod utils;

fn a<'a, T: 'a>(x: &impl DataReadBuf<Item = u8>) {
    let rx = x as &dyn DataReadBuf<Item = u8>;
}

#[derive(Debug)]
struct A<'a> {
    lmao: &'a mut String,
}

impl<'a> A<'a> {
    pub fn uwu(&mut self, x: &'a mut String) -> &'a mut String {
        // self.lmao = x;
        x
    }
}

fn b() {
    let mut a = "asdasd".to_owned();
    let mut b = "asda".to_owned();
    let mut aa = A { lmao: &mut a };
    {
        aa.uwu(&mut b);
    }

    dbg!(aa);
}
