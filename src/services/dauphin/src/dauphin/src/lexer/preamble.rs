pub const PREAMBLE: &str = r#"

func __star__;
func __sqopen__;
func __dot__;
func __query__;
func __pling__;
func __ref__;
func __sqctor__;
inline "*" __star__ prefix 8;
inline "[" __sqopen__ suffix 4;
inline "[" __sqctor__ prefix 4;
inline "." __dot__ suffix 4;
inline "?" __query__ suffix 4;
inline "!" __pling__ suffix 4;
inline "&[" __ref__ suffix 4;

"#;