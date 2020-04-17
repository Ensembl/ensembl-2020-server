use serde_cbor::Value as CborValue;

pub fn cbor_int(cbor: &CborValue, max: Option<i128>) -> Result<i128,String>  {
    match cbor {
        CborValue::Integer(x) => {
            if let Some(max) = max {
                if *x >= 0 && *x <= max { return Ok(*x); }
            } else {
                return Ok(*x);
            }
        },
        _ => {}
    }
    Err(format!("bad cbor: expected int, unexpected {:?}",cbor))
}

pub fn cbor_string(cbor: &CborValue) -> Result<String,String> {
    match cbor {
        CborValue::Text(x) => Ok(x.to_string()),
        _ => Err(format!("bad cbor: expected string, unexpected {:?}",cbor))
    }
}

pub fn cbor_array(cbor: &CborValue, len: usize, or_more: bool) -> Result<Vec<CborValue>,String> {
    match cbor {
        CborValue::Array(a) => {
            if a.len() == len || (a.len() >= len && or_more) {
                return Ok(a.to_vec());
            }
        },
        _ => {}
    }
    Err(format!("bad cbor: expected array len={:?}/{:?}, unexpected {:?}",len,or_more,cbor))
}