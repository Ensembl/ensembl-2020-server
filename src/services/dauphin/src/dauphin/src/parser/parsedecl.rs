use crate::lexer::{ Lexer, Token };
use super::node::{ ParserStatement, ParseError };
use crate::codegen::DefStore;
use crate::types::{ Type, BaseType, TypeSig, Sig, TypeSigExpr };
use super::lexutil::{ get_other, get_identifier };

pub(in super) fn parse_exprdecl(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    Ok(ParserStatement::ExprMacro(name.to_string()))
}

pub(in super) fn parse_stmtdecl(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    Ok(ParserStatement::StmtMacro(name.to_string()))
}

pub(in super) fn parse_func(lexer: &mut Lexer, defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    let mut srcs = Vec::new();
    get_other(lexer,"(")?;
    loop {
        if lexer.peek() == Token::Other(')') { lexer.get(); break; }
        srcs.push(parse_typesigexpr(lexer,defstore)?);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    if get_identifier(lexer)? != "becomes" {
        return Err(ParseError::new("missing 'becomes'",lexer));
    }
    let dst = parse_typesigexpr(lexer,defstore)?;
    Ok(ParserStatement::FuncDecl(name.to_string(),dst,srcs))
}

pub(in super) fn parse_proc(lexer: &mut Lexer,defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    let mut sigs = Vec::new();
    get_other(lexer,"(")?;
    loop {
        if lexer.peek() == Token::Other(')') { break; }
        sigs.push(parse_signature(lexer,defstore)?);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    Ok(ParserStatement::ProcDecl(name.to_string(),sigs))
}

pub fn parse_signature(lexer: &mut Lexer, defstore: &DefStore) -> Result<Sig,ParseError> {
    let mut lvalue = false;
    let mut out = false;
    loop {
        match lexer.peek() {
            Token::Identifier(name) => {
                match &name[..] {
                    "lvalue" => {
                        lvalue = true;
                        lexer.get();
                    },
                    "out" => {
                        out = true;
                        lexer.get();
                    },
                    _ => break
                }
            },
            _ => ()
        }
    }
    let fromsig = parse_typesig(lexer,defstore)?;
    let (fromsig,lvalue) = if out {
        (TypeSig::Right(TypeSigExpr::Placeholder("_".to_string())),Some(fromsig.expr().clone()))
    } else {
        (fromsig,None)
    };
    let lvalue = lvalue.map(|x| TypeSig::Right(x));
    Ok(Sig { lvalue, out, typesig: fromsig })
}

fn id_to_type(id: &str, lexer: &Lexer, defstore: &DefStore) -> Result<BaseType,ParseError> {
    if defstore.get_struct(id).is_some() {
        Ok(BaseType::StructType(id.to_string()))
    } else if defstore.get_enum(id).is_some() {
        Ok(BaseType::EnumType(id.to_string()))
    } else {
        Err(ParseError::new(&format!("No such struct/enum '{}'",id),lexer))
    }
}

fn parse_type(lexer: &mut Lexer, defstore: &DefStore) -> Result<Type,ParseError> {
    Ok(match &get_identifier(lexer)?[..] {
        "boolean" => Type::Base(BaseType::BooleanType),
        "number" => Type::Base(BaseType::NumberType),
        "string" => Type::Base(BaseType::StringType),
        "bytes" => Type::Base(BaseType::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out =  Type::Vector(Box::new(parse_type(lexer,defstore)?));
            get_other(lexer,")")?;
            out
        },
        x => Type::Base(id_to_type(x,lexer,defstore)?)
    })
}

pub fn parse_typesig(lexer: &mut Lexer, defstore: &DefStore) -> Result<TypeSig,ParseError> {
    Ok(TypeSig::Right(parse_typesigexpr(lexer,defstore)?))
}

pub fn parse_typesigexpr(lexer: &mut Lexer, defstore: &DefStore) -> Result<TypeSigExpr,ParseError> {
    Ok(match &get_identifier(lexer)?[..] {
        "boolean" => TypeSigExpr::Base(BaseType::BooleanType),
        "number" => TypeSigExpr::Base(BaseType::NumberType),
        "string" => TypeSigExpr::Base(BaseType::StringType),
        "bytes" => TypeSigExpr::Base(BaseType::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out =  TypeSigExpr::Vector(Box::new(parse_typesigexpr(lexer,defstore)?));
            get_other(lexer,")")?;
            out
        },
        x => {
            if x.starts_with("_") {
                TypeSigExpr::Placeholder(x.to_string())
            } else {
                TypeSigExpr::Base(id_to_type(x,lexer,defstore)?)
            }
        }
    })
}

fn parse_struct_short(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<Type>,Vec<String>),ParseError> {
    let mut out = Vec::new();
    loop {
        out.push(parse_type(lexer,defstore)?);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        };
    }
    let len = out.len();
    Ok((out,(0..len).into_iter().map(|x| (x.to_string())).collect()))
}

fn parse_struct_enum_full(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<Type>,Vec<String>),ParseError> {
    let mut out = Vec::new();
    let mut names = Vec::new();
    if let Token::Other('}') = lexer.peek() {
        lexer.get();
        return Ok((vec![],vec![]));
    }
    loop {
        names.push(get_identifier(lexer)?);
        get_other(lexer,":")?;
        out.push(parse_type(lexer,defstore)?);        
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        }
    }
    Ok((out,names))
}

fn parse_struct_contents(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<Type>,Vec<String>),ParseError> {
    let start = lexer.pos();
    Ok(match lexer.get() {
        Token::Identifier(_) => {
            let next = lexer.peek().clone();
            lexer.back_to(start);
            match next {
                Token::Other(':') => parse_struct_enum_full(lexer,defstore)?,
                _ => parse_struct_short(lexer,defstore)?
            }
        },
        Token::Other('}') => {
            (vec![],vec![])
        },
        _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
    })
}

pub(in super) fn parse_struct(lexer: &mut Lexer, defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    get_other(lexer, "{")?;
    let (types,names) = parse_struct_contents(lexer,defstore)?;
    Ok(ParserStatement::StructDef(name.to_string(),types,names))
}

pub(in super) fn parse_enum(lexer: &mut Lexer, defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    get_other(lexer, "{")?;
    let (types,names) = parse_struct_enum_full(lexer,defstore)?;
    Ok(ParserStatement::EnumDef(name.to_string(),types,names))
}
