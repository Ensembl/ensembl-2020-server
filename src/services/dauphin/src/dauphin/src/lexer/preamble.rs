pub const PREAMBLE: &str = r#"

func __star__() becomes _;
func __sqopen__() becomes _;
func __dot__() becomes _;
func __query__() becomes _;
func __pling__() becomes _;
func __ref__() becomes _;
func __sqctor__() becomes _;
inline "*" __star__ prefix 8;
inline "[" __sqopen__ suffix 4;
inline "[" __sqctor__ prefix 4;
inline "." __dot__ suffix 4;
inline "?" __query__ suffix 4;
inline "!" __pling__ suffix 4;
inline "&[" __ref__ suffix 4;

"#;