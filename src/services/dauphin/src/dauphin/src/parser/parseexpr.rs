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
 *  
 *  vscode-fold=1
 */

use crate::lexer::{ Lexer, Token };
use crate::model::{ DefStore, InlineMode };
use super::node::{ ParseError, Expression };
use super::lexutil::{get_other, get_identifier };

fn vec_ctor(lexer: &mut Lexer, defstore: &DefStore, nested: bool) -> Result<Expression,ParseError> {
    Ok(Expression::Vector(parse_exprlist(lexer,defstore,']',nested)?))
}

fn parse_prefix(lexer: &mut Lexer, defstore: &DefStore, op: &str, nested: bool) -> Result<Expression,ParseError> {
    if defstore.stmt_like(op,lexer).unwrap_or(false) { /* stmt-like */
        return Err(ParseError::new("Unexpected statement",lexer));
    }
    let inline = defstore.get_inline_unary(op,lexer)?;
    let prec = inline.precedence();
    if inline.mode() != &InlineMode::Prefix {
        return Err(ParseError::new("Not a prefix operator",lexer));
    }
    let name = inline.name().to_string();
    Ok(match &name[..] {
        "__star__" => Expression::Star(Box::new(parse_expr_level(lexer,defstore,Some(prec),true,nested)?)),
        "__sqctor__" => vec_ctor(lexer,defstore,nested)?,
        _ => Expression::Operator(name.to_string(),vec![parse_expr_level(lexer,defstore,Some(prec),true,nested)?])
    })
}

fn require_filter(lexer: &mut Lexer, c: char, nested: bool) -> Result<(),ParseError> {
    if !nested {
        return Err(ParseError::new(&format!("{} encountered outside filter",c),lexer));
    }
    Ok(())
}

fn make_names(len: usize) -> Vec<String> {
    (0..len).map(|v| v.to_string()).collect()
}

fn parse_struct_ctor(lexer: &mut Lexer, defstore: &DefStore, id: &str, nested: bool) -> Result<Expression,ParseError> {
    get_other(lexer,"{")?;
    let pos = lexer.pos();
    if let Token::Identifier(_) = lexer.get() {
        if lexer.peek() == Token::Other(':') {
            lexer.back_to(pos);
            return parse_ctor_full(lexer,defstore,id,nested);
        }
    }
    let inner = parse_exprlist(lexer,defstore,'}',nested)?;
    let names = make_names(inner.len());
    return Ok(Expression::CtorStruct(id.to_string(),inner,names));
}

fn parse_ctor_full(lexer: &mut Lexer, defstore: &DefStore, id: &str, nested: bool) -> Result<Expression,ParseError> {
    let mut inner = Vec::new();
    let mut names = Vec::new();
    if let Token::Other('}') = lexer.peek() {
        lexer.get();
        return Ok(Expression::CtorStruct(id.to_string(),vec![],vec![]));
    }
    loop {
        names.push(get_identifier(lexer)?);
        get_other(lexer,":")?;
        inner.push(parse_expr(lexer,defstore,nested)?);        
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        }
    }
    Ok(Expression::CtorStruct(id.to_string(),inner,names))
}

fn parse_atom_id(lexer: &mut Lexer, defstore: &DefStore, id: &str, nested: bool) -> Result<Expression,ParseError> {
    if defstore.stmt_like(id,lexer).unwrap_or(false) {
        Err(ParseError::new("Unexpected statement in expression",lexer))?;
    }
    if !defstore.stmt_like(id,lexer).unwrap_or(true) { /* expr-like */
        get_other(lexer, "(")?;
        Ok(Expression::Operator(id.to_string(),parse_exprlist(lexer,defstore,')',nested)?))
    } else {
        Ok(match id {
            "true" => Expression::LiteralBool(true),
            "false" => Expression::LiteralBool(false),
            id => Expression::Identifier(id.to_string())
        })
    }
}

fn peek_enum_ctor(lexer: &mut Lexer) -> bool {
    let pos = lexer.pos();
    let x = lexer.get();
    let y = lexer.peek();
    let out = if let Token::Identifier(_) = y {
        x == Token::Other(':')
    } else {
        false
    };
    lexer.back_to(pos);
    out
}

fn parse_atom(lexer: &mut Lexer, defstore: &DefStore, nested: bool) -> Result<Expression,ParseError> {
    Ok(match lexer.get_oper(true) {
        Token::Identifier(id) => {
            if lexer.peek() == Token::Other('{') {
                parse_struct_ctor(lexer,defstore,&id,nested)?
            } else if peek_enum_ctor(lexer) {
                lexer.get();
                let branch = get_identifier(lexer)?;
                let expr = parse_expr(lexer,defstore,nested)?;
                Expression::CtorEnum(id.to_string(),branch.to_string(),Box::new(expr))
            } else {
                parse_atom_id(lexer,defstore,&id,nested)?
            }
        },
        Token::Number(num,_) => Expression::Number(num),
        Token::LiteralString(s) => Expression::LiteralString(s),
        Token::LiteralBytes(b) => Expression::LiteralBytes(b),
        Token::Other('(') => {
            let out = parse_expr(lexer,defstore,nested)?;
            get_other(lexer,")")?;
            out
        },
        Token::Other('$') => {
            require_filter(lexer,'$',nested)?;
            Expression::Dollar
        },
        Token::Other('@') => {
            require_filter(lexer,'@',nested)?;
            Expression::At
        },
        Token::Operator(op) => parse_prefix(lexer,defstore,&op,nested)?,
        x => Err(ParseError::new(&format!("Expected expression, not {:?}",x),lexer))?
    })
}

fn parse_brackets(lexer: &mut Lexer, defstore: &DefStore, left: Expression) -> Result<Expression,ParseError> {
    if let Token::Other(']') = lexer.peek() {
        lexer.get();
        Ok(Expression::Square(Box::new(left)))
    } else {
        let inside = parse_expr(lexer,defstore,true)?;
        get_other(lexer, "]")?;
        Ok(Expression::Bracket(Box::new(left),Box::new(inside)))
    }
}

fn parse_suffix(lexer: &mut Lexer, defstore: &DefStore, left: Expression, name: &str) -> Result<Expression,ParseError> {
    lexer.get_oper(false);
    Ok(match &name[..] {
        "__sqopen__" => parse_brackets(lexer,defstore,left)?,
        "__dot__" => Expression::Dot(Box::new(left),get_identifier(lexer)?),
        "__query__" => Expression::Query(Box::new(left),get_identifier(lexer)?),
        "__pling__" => Expression::Pling(Box::new(left),get_identifier(lexer)?),
        "__ref__" => {
            if let Expression::Bracket(op,key) = parse_brackets(lexer,defstore,left)? {
                return Ok(Expression::Filter(op,key));
            } else {
                return Err(ParseError::new("Expected filter",lexer));
            }
        },
        _ => Expression::Operator(name.to_string(),vec![left])
    })
}

fn parse_binary_right(lexer: &mut Lexer, defstore: &DefStore, left: Expression, name: &str, min: f64, oreq: bool, nested: bool) -> Result<Expression,ParseError> {
    lexer.get_oper(false);
    let right = parse_expr_level(lexer,defstore,Some(min),oreq,nested)?;
    Ok(Expression::Operator(name.to_string(),vec![left,right]))
}

fn extend_expr(lexer: &mut Lexer, defstore: &DefStore, left: Expression, symbol: &str, min: Option<f64>, oreq: bool, nested: bool) -> Result<(Expression,bool),ParseError> {
    let inline = defstore.get_inline_binary(symbol,lexer)?;
    let prio = inline.precedence();
    if let Some(min) = min {
        if prio > min || (prio==min && !oreq) {
            return Ok((left,false));
        }
    }
    let name = inline.name().to_string();
    if defstore.stmt_like(&name,lexer)? {
        return Ok((left,false));
    }
    Ok(match *inline.mode() {
        InlineMode::LeftAssoc => (parse_binary_right(lexer,defstore,left,&name,prio,false,nested)?,true),
        InlineMode::RightAssoc => (parse_binary_right(lexer,defstore,left,&name,prio,true,nested)?,true),
        InlineMode::Prefix => (left,false),
        InlineMode::Suffix => (parse_suffix(lexer,defstore,left,&name)?,true)
    })
}

fn parse_expr_level(lexer: &mut Lexer, defstore: &DefStore, min: Option<f64>, oreq: bool, nested: bool) -> Result<Expression,ParseError> {
    let mut out = parse_atom(lexer,defstore,nested)?;
    loop {
        match lexer.peek_oper(false) {
            Token::Operator(op) => {
                let op = op.to_string();
                let (expr,progress) = extend_expr(lexer,defstore,out,&op,min,oreq,nested)?;
                out = expr;
                if !progress {
                    return Ok(out);
                }
            },
            _ => return Ok(out)
        }
    }
}

pub(in super) fn parse_expr(lexer: &mut Lexer, defstore: &DefStore, nested: bool) -> Result<Expression,ParseError> {
    parse_expr_level(lexer,defstore,None,true,nested)
}

pub(in super) fn parse_exprlist(lexer: &mut Lexer, defstore: &DefStore, term: char, nested: bool) -> Result<Vec<Expression>,ParseError> {
    let mut out = Vec::new();
    loop {
        match lexer.peek() {
            Token::Other(x) if x == term => {
                lexer.get();
                return Ok(out)
            },
            Token::Other(',') => {
                lexer.get();
            },
            _ => {
                out.push(parse_expr(lexer,defstore,nested)?);
            }
        }
    }
}
