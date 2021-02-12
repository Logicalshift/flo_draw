#![allow(warnings)]

include!(concat!(env!("OUT_DIR"), "/gbm.rs"));

#[link(name = "gbm")]
extern {}
