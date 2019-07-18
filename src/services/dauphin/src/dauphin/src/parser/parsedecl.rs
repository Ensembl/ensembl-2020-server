use crate::lexer::{ Lexer, Token };
use super::node::{ ParserStatement, ParseError, Type, BaseType, TypeSig, Sig, TypeSigExpr };
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

pub(in super) fn parse_func(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    Ok(ParserStatement::FuncDecl(name.to_string()))
}

pub(in super) fn parse_proc(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    let mut sigs = Vec::new();
    get_other(lexer,"(")?;
    loop {
        sigs.push(parse_signature(lexer)?);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    Ok(ParserStatement::ProcDecl(name.to_string(),sigs))
}

fn parse_signature(lexer: &mut Lexer) -> Result<Sig,ParseError> {
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
    Ok(Sig { lvalue, out, reverse: false, typesig: parse_typesig(lexer)? })
}

fn parse_type(lexer: &mut Lexer) -> Result<Type,ParseError> {
    Ok(match &get_identifier(lexer)?[..] {
        "boolean" => Type::Base(BaseType::BooleanType),
        "number" => Type::Base(BaseType::NumberType),
        "string" => Type::Base(BaseType::StringType),
        "bytes" => Type::Base(BaseType::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out =  Type::Vector(Box::new(parse_type(lexer)?));
            get_other(lexer,")")?;
            out
        },
        x => Type::Base(BaseType::IdentifiedType(x.to_string()))
    })
}

pub fn parse_typesig(lexer: &mut Lexer) -> Result<TypeSig,ParseError> {
    Ok(TypeSig::Right(parse_typesigexpr(lexer)?))
}

pub fn parse_typesigexpr(lexer: &mut Lexer) -> Result<TypeSigExpr,ParseError> {
    Ok(match &get_identifier(lexer)?[..] {
        "boolean" => TypeSigExpr::Base(BaseType::BooleanType),
        "number" => TypeSigExpr::Base(BaseType::NumberType),
        "string" => TypeSigExpr::Base(BaseType::StringType),
        "bytes" => TypeSigExpr::Base(BaseType::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out =  TypeSigExpr::Vector(Box::new(parse_typesigexpr(lexer)?));
            get_other(lexer,")")?;
            out
        },
        x => {
            if x.starts_with("_") {
                TypeSigExpr::Placeholder(x.to_string())
            } else {
                TypeSigExpr::Base(BaseType::IdentifiedType(x.to_string()))
            }
        }
    })
}

fn parse_struct_short(lexer: &mut Lexer) -> Result<(Vec<Type>,Vec<String>),ParseError> {
    let mut out = Vec::new();
    loop {
        out.push(parse_type(lexer)?);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        };
    }
    let len = out.len();
    Ok((out,(0..len).into_iter().map(|x| (x.to_string())).collect()))
}

fn parse_struct_enum_full(lexer: &mut Lexer) -> Result<(Vec<Type>,Vec<String>),ParseError> {
    let mut out = Vec::new();
    let mut names = Vec::new();
    if let Token::Other('}') = lexer.peek() {
        lexer.get();
        return Ok((vec![],vec![]));
    }
    loop {
        names.push(get_identifier(lexer)?);
        get_other(lexer,":")?;
        out.push(parse_type(lexer)?);        
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        }
    }
    Ok((out,names))
}

fn parse_struct_contents(lexer: &mut Lexer) -> Result<(Vec<Type>,Vec<String>),ParseError> {
    let start = lexer.pos();
    Ok(match lexer.get() {
        Token::Identifier(_) => {
            let next = lexer.peek().clone();
            lexer.back_to(start);
            match next {
                Token::Other(':') => parse_struct_enum_full(lexer)?,
                _ => parse_struct_short(lexer)?
            }
        },
        Token::Other('}') => {
            (vec![],vec![])
        },
        _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
    })
}

pub(in super) fn parse_struct(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    get_other(lexer, "{")?;
    let (types,names) = parse_struct_contents(lexer)?;
    Ok(ParserStatement::StructDef(name.to_string(),types,names))
}

pub(in super) fn parse_enum(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    get_other(lexer, "{")?;
    let (types,names) = parse_struct_enum_full(lexer)?;
    Ok(ParserStatement::EnumDef(name.to_string(),types,names))
}
