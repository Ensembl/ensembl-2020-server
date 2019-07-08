/* Despite looking like a parser concern, operators are lexer tokens and so the validity of an
 * operator is a lexer tihng and so a left-ro-tight thing. This means that prefix operators, which
 * can occur whenever an expression is expected, are distinct from infix or postfix operators which
 * occur after an expression and extend it. However infix and postfix operators are essentially
 * equivalent in terms of detection, differeing only in what happens next after detection.
 * 
 * NO operator may contain "//"" slash-star or ";". The former are comment characters and the latter
 * is the statement terminsation character which is used to fast-forward through eroneous statements.
 * No operator can contain whitespace nor be empty.
 * 
 * Avioding bracketing ambiguiity: Operators may not be defined such that a sequence of them may
 * be bracketed ambiguosuly. To achieve this none may be a prefix of any other. This condition may
 * be later relaxed while maintaining the assurance.
 * 
 * Let A/N be alphanumerics or underscore.
 * 
 * It must begin #%*+-/<=>\^`{|}~&*" or !?. followed by a non-A/N. Alternatively, it may be bracketed on 
 * the outside with () or [] if it contains only such characters. Additionally, if bracketed may begin
 * with !?. Note that &[ and * themselves are registered as operators in the preamble.
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
    /* cannot be the prefix of any other */
    if tokens.is_prefix_of(c) {
        return Err("one operator cannot be a prefix of another".to_string());
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
        assert_eq!("one operator cannot be a prefix of another",
                    check_inline(&it,"(+)+",false).expect_err("(+)+"));
        it.add("(+)(*)",false).expect_err("(+)(*)");
        it.add("&hello&",false).expect("(+)");
        it.add(".fred",false).expect_err(".fred");
        it.add("..",false).expect("..");
        it.add("(.)",false).expect("(.)");
        it.add("(.f)",false).expect("(.f)");
        it.add("&he",false).expect_err("&he");
    }
}