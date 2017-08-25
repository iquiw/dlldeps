use std::str::Utf8Error;
use pelite;

error_chain!{
    foreign_links {
        PE(pelite::Error);
        Utf8(Utf8Error);
    }
}
