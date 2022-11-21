#[macro_export]
pub mod ski_lang;
pub mod skpicture;

pub mod protos {
    include!(concat!(env!("OUT_DIR"), "/ski_pass.rs"));
}
