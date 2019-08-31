use crate::lexer::{ Lexer, Token };
use super::node::{ ParserStatement, ParseError };
use crate::codegen::DefStore;
use crate::types::{ Type, BaseType, TypeSig, Sig, TypeSigExpr };
use crate::typeinf::{ ArgumentExpressionConstraint, SignatureConstraint, SignatureMemberConstraint, MemberType };
use crate::typeinf::BaseType as BaseType2;
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
    let mut members = Vec::new();
    let name = get_identifier(lexer)?;
    get_other(lexer,"(")?;
    loop {
        if lexer.peek() == Token::Other(')') { lexer.get(); break; }
        members.push(SignatureMemberConstraint::RValue(parse_typesigexpr(lexer,defstore)?.1));
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    if get_identifier(lexer)? != "becomes" {
        return Err(ParseError::new("missing 'becomes'",lexer));
    }
    members.insert(0,SignatureMemberConstraint::RValue(parse_typesigexpr(lexer,defstore)?.1));
    Ok(ParserStatement::FuncDecl(name.to_string(),SignatureConstraint::new(&members),TypeSigExpr::Base(BaseType::Invalid),vec![]))
}

pub(in super) fn parse_proc(lexer: &mut Lexer,defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    let mut sigs = Vec::new();
    let mut members = Vec::new();
    get_other(lexer,"(")?;
    loop {
        if lexer.peek() == Token::Other(')') { break; }
        let (sig,member) = parse_signature(lexer,defstore)?;
        sigs.push(sig);
        members.push(member);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    Ok(ParserStatement::ProcDecl(name.to_string(),sigs,SignatureConstraint::new(&members)))
}

pub fn parse_signature(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Sig,SignatureMemberConstraint),ParseError> {
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
    let (fromsig,argtype) = parse_typesig(lexer,defstore)?;
    let member = if out {
        SignatureMemberConstraint::LValue(argtype)
    } else {
        SignatureMemberConstraint::RValue(argtype)
    };
    let (fromsig,lvalue) = if out {
        (TypeSig::Right(TypeSigExpr::Placeholder("_".to_string())),Some(fromsig.expr().clone()))
    } else {
        (fromsig,None)
    };
    let lvalue = lvalue.map(|x| TypeSig::Right(x));
    Ok((Sig { lvalue, out, typesig: fromsig },member))
}

fn id_to_type(id: &str, lexer: &Lexer, defstore: &DefStore) -> Result<(BaseType,BaseType2),ParseError> {
    if defstore.get_struct(id).is_some() {
        Ok((BaseType::StructType(id.to_string()),BaseType2::StructType(id.to_string())))
    } else if defstore.get_enum(id).is_some() {
        Ok((BaseType::EnumType(id.to_string()),BaseType2::EnumType(id.to_string())))
    } else {
        Err(ParseError::new(&format!("No such struct/enum '{}'",id),lexer))
    }
}

fn parse_type(lexer: &mut Lexer, defstore: &DefStore) -> Result<(MemberType,Type),ParseError> {
    let id = get_identifier(lexer)?;
    let new = match &id[..] {
        "boolean" => MemberType::Base(BaseType2::BooleanType),
        "number" => MemberType::Base(BaseType2::NumberType),
        "string" => MemberType::Base(BaseType2::StringType),
        "bytes" => MemberType::Base(BaseType2::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out = MemberType::Vec(Box::new(parse_type(lexer,defstore)?.0));
            get_other(lexer,")")?;
            out
        },
        x => MemberType::Base(id_to_type(x,lexer,defstore)?.1)
    };
    // XXX switch
    Ok((new,Type::Base(BaseType::BooleanType)))
}

pub fn parse_typesig(lexer: &mut Lexer, defstore: &DefStore) -> Result<(TypeSig,ArgumentExpressionConstraint),ParseError> {
    let (sig,expr_constr) = parse_typesigexpr(lexer,defstore)?;
    Ok((TypeSig::Right(sig),expr_constr))
}

pub fn parse_typesigexpr(lexer: &mut Lexer, defstore: &DefStore) -> Result<(TypeSigExpr,ArgumentExpressionConstraint),ParseError> {
    let id = get_identifier(lexer)?;
    let sig = match &id[..] {
        "boolean" => TypeSigExpr::Base(BaseType::BooleanType),
        "number" => TypeSigExpr::Base(BaseType::NumberType),
        "string" => TypeSigExpr::Base(BaseType::StringType),
        "bytes" => TypeSigExpr::Base(BaseType::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out =  TypeSigExpr::Vector(Box::new(parse_typesigexpr(lexer,defstore)?.0));
            get_other(lexer,")")?;
            out
        },
        x => {
            if x.starts_with("_") {
                TypeSigExpr::Placeholder(x.to_string())
            } else {
                TypeSigExpr::Base(id_to_type(x,lexer,defstore)?.0)
            }
        }
    };
    let constraint = match &id[..] {
        "boolean" => ArgumentExpressionConstraint::Base(BaseType2::BooleanType),
        "number" => ArgumentExpressionConstraint::Base(BaseType2::NumberType),
        "string" => ArgumentExpressionConstraint::Base(BaseType2::StringType),
        "bytes" => ArgumentExpressionConstraint::Base(BaseType2::BytesType),
        "vec" => {
            get_other(lexer,"(")?;
            let out =  ArgumentExpressionConstraint::Vec(Box::new(parse_typesigexpr(lexer,defstore)?.1));
            get_other(lexer,")")?;
            out
        },
        x => {
            if x.starts_with("_") {
                ArgumentExpressionConstraint::Placeholder(x.to_string())
            } else {
                ArgumentExpressionConstraint::Base(id_to_type(x,lexer,defstore)?.1)
            }
        }
    };
    Ok((sig,constraint))
}

fn parse_struct_short(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<MemberType>,Vec<Type>,Vec<String>),ParseError> {
    let mut types = Vec::new();
    let mut out = Vec::new();
    loop {
        let (member_type,type_) = parse_type(lexer,defstore)?;
        out.push(type_);
        types.push(member_type);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        };
    }
    let len = types.len();
    Ok((types,out,(0..len).into_iter().map(|x| (x.to_string())).collect()))
}

fn parse_struct_enum_full(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<MemberType>,Vec<Type>,Vec<String>),ParseError> {
    let mut out = Vec::new();
    let mut types = Vec::new();
    let mut names = Vec::new();
    if let Token::Other('}') = lexer.peek() {
        lexer.get();
        return Ok((vec![],vec![],vec![]));
    }
    loop {
        names.push(get_identifier(lexer)?);
        get_other(lexer,":")?;
        let (member_type,type_) = parse_type(lexer,defstore)?;
        out.push(type_);
        types.push(member_type);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        }
    }
    Ok((types,out,names))
}

fn parse_struct_contents(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<MemberType>,Vec<Type>,Vec<String>),ParseError> {
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
            (vec![],vec![],vec![])
        },
        _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
    })
}

pub(in super) fn parse_struct(lexer: &mut Lexer, defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    get_other(lexer, "{")?;
    let (member_types,types,names) = parse_struct_contents(lexer,defstore)?;
    Ok(ParserStatement::StructDef(name.to_string(),member_types,types,names))
}

pub(in super) fn parse_enum(lexer: &mut Lexer, defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let name = get_identifier(lexer)?;
    get_other(lexer, "{")?;
    let (member_types,types,names) = parse_struct_enum_full(lexer,defstore)?;
    Ok(ParserStatement::EnumDef(name.to_string(),member_types,types,names))
}