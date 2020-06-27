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

use super::inlinetokens::InlineTokens;

const SPECIAL : &str = "\"$'(),;[]@";
const NONALNUM : &str = ".?!:";

fn check_unbracketed(c: &str) -> Result<(),String> {
    let mut chars = c.chars();
    let first = chars.next().unwrap();
    let second = chars.next();
    if SPECIAL.contains(first) {
        return Err(format!("operator cannot have '{}' as first character",first));
    }
    if NONALNUM.contains(first) {
        if let Some(second) = second {
            if second.is_alphanumeric() || second == '_' {
                return Err(format!("if operator begins with '{}' it must be followed by non-alnum not '{}'",first,second));
            }
        }
    }
    Ok(())
}

fn check_regular_bracketed(c: &str) -> Result<(),String> {
    let mut chars = c.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch.is_alphanumeric() || ch == '_' {
            return Err(format!("operator cannot contain '{}'",ch));
        }
        if NONALNUM.contains(ch) {
            if let Some(next) = chars.peek() {
                if next.is_alphanumeric() || next == &'_' {
                    return Err(format!("'{}' must be followed by non-alnum in operator",ch));
                }
            }
        } else if SPECIAL.contains(ch) {
            return Err(format!("operator cannot contain '{}'",ch));
        }
    }
    Ok(())
}

fn check_bracketed(c: &str) -> Result<(),String> {
    if c.len() == 0 {
        return Err("operator cannot only be brackets".to_string());
    }
    let first = c.chars().next().unwrap();
    let last = c.chars().last().unwrap();
    if (first == '(' && last == ')') || (first == '[' && last == ']') {
        check_bracketed(&c[1..c.len()-1])?;
    } else if !NONALNUM.contains(first) {
        check_regular_bracketed(c)?;
    }
    Ok(())
}

pub fn check_inline(tokens: &InlineTokens, c: &str, prefix: bool) -> Result<(),String> {
    /* cannot contain slash-star, slash-slash, semicolon */
    for b in &vec!["//","/*",";"] {
        if c.contains(b) {
            return Err(format!("operator '{}' invalid, cannot contain '{}'",c,b));
        }
    }
    /* cannot contain whitespace */
    for c in c.chars() {
        if c.is_whitespace() {
            return Err(format!("operator '{}' invalid, cannot contain whitespace",c));
        }
    }
    /* cannot begin with alphanumerics or be blank */
    if let Some(c) = c.chars().next() {
        if c.is_alphanumeric() || c == '_' {
            return Err("operator cannot begin with alphanumeric".to_string());
        }
    } else {
        return Err("operator cannot be blank".to_string());
    }
    /* cannot register an operator twice except as prefix and other */
    if tokens.equal(c,prefix) {
        return Err("operator already defined".to_string());
    }
    /* character check */
    if let Some(first) = c.chars().next() {
        if first == '(' || first == '[' && c != "[" {
            check_bracketed(c)?;
        } else if c != "[" && c != "&[" && c != "?" && c != "!" && c != "." {
            check_unbracketed(c)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_smoke() {
        let mut it = InlineTokens::new();
        it.add("(*)",true).expect("(*)");
        it.add("(+)",false).expect("(+)");
        it.add("(+)",false).expect_err("(+)");
        it.add("(+)(*)",false).expect_err("(+)(*)");
        it.add("&hello&",false).expect("(+)");
        it.add(".fred",false).expect_err(".fred");
        it.add("..",false).expect("..");
        it.add("(.)",false).expect("(.)");
        it.add("(.f)",false).expect("(.f)");
    }
}