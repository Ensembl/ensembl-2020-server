/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use crate::cli::Config;
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
