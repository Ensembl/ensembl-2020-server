/* 
 *  This is the default license template.
 *  
 *  File: lib.rs
 *  Author: dan
 *  Copyright (c) 2020 dan
 *  
 *  To edit this license information: Press Ctrl+Shift+P and press 'Create new License Template...'.
 */

mod commands {
    mod buildtime;
    mod defines;
    mod dump;
    mod hints;
    mod ini;
    mod versions;

    pub use self::buildtime::make_buildtime;
}

pub use self::commands::make_buildtime;

#[cfg(test)]
mod test;