/* 
 *  This is the default license template.
 *  
 *  File: config.rs
 *  Author: dan
 *  Copyright (c) 2020 dan
 *  
 *  To edit this license information: Press Ctrl+Shift+P and press 'Create new License Template...'.
 */

use dauphin_compile::cli::Config;
use crate::test::files::{ find_testdata };

pub fn xxx_test_config() -> Config {
    let mut cfg = Config::new();
    cfg.set_root_dir(&find_testdata().to_string_lossy());
    cfg.set_generate_debug(true);
    cfg.set_unit_test(true);
    cfg.set_verbose(3);
    cfg.set_opt_level(2);
    cfg.set_debug_run(true);
    cfg.add_lib("buildtime");
    cfg.add_file_search_path("*.dp");
    cfg.add_file_search_path("parser/*.dp");
    cfg.add_file_search_path("parser/import-subdir/*.dp");
    cfg
}
