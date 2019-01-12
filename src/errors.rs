use pelite;
use std::io;
use std::str::Utf8Error;

error_chain! {
    foreign_links {
        IO(io::Error);
        PE(pelite::Error);
        Utf8(Utf8Error);
    }
}
