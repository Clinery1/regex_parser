use std::{
    fs::read_to_string,
    time::Instant,
    num::ParseIntError,
};
use regex_parser2::*;


#[derive(Debug)]
enum Stmt<'a> {
    Assign {
        name:&'a str,
        num:usize,
    },
    Declare {
        name:&'a str,
        num:usize,
    },
    Block(Vec<Self>),
}
#[derive(Debug)]
enum Error {
    NumParse(String),
    InvalidMatch,
    Multiple(Vec<Self>),
}
impl From<RegexParserError> for Error {
    fn from(_:RegexParserError)->Self {
        Error::InvalidMatch
    }
}
impl From<Vec<Self>> for Error {
    fn from(errs:Vec<Self>)->Self {
        Self::Multiple(errs)
    }
}
impl From<ParseIntError> for Error {
    fn from(err:ParseIntError)->Self {
        Self::NumParse(err.to_string())
    }
}


fn main() {
    let data=read_to_string("example").unwrap();
    let parser_create_start=Instant::now();
    let word_parser=Map::new(
        r"[a-zA-Z_][a-zA-Z0-9_]*".into_parser(),
        |name_res:Result<_,Error>|match name_res {
            Ok(name)=>Ok(name[0].matched),
            Err(e)=>Err(e),
        },
    );
    let usize_parser=number_parser::<usize,Error>();
    let equal_parser="=".into_parser();
    let ws_parser="[ \t\r]+".into_parser();
    let opt_ws_parser="[ \t\r]*".into_parser();
    let nl_parser="\n+".into_parser();
    let eof_parser=r"\z".into_parser();
    let let_kw_parser="let".into_parser();
    let opt_nl_or_ws_parser="[ \r\t\n]*".into_parser();
    let nl_or_eof_parser=(
        &nl_parser,
        &eof_parser,
    )
        .any()
        .map(|res|match res {
            Ok(o)=>Ok(o),
            Err(mut e)=>Err(e.remove(0)),
        });
    let assign_parser=(
        &word_parser,
        &opt_ws_parser,
        &equal_parser,
        &opt_ws_parser,
        &usize_parser,
    ).map(|res|match res {
        Ok((name,_,_,_,num))=>Ok(Stmt::Assign{name,num}),
        Err(e)=>Err(e),
    });
    let declare_parser=(
        &let_kw_parser,
        &ws_parser,
        &word_parser,
        &opt_ws_parser,
        &equal_parser,
        &opt_ws_parser,
        &usize_parser,
    ).map(|res|match res {
        Ok((_,_,name,_,_,_,num))=>Ok(Stmt::Declare{name,num}),
        Err(e)=>Err(e),
    });
    let stmt_inner_parser=(
        &declare_parser,
        &assign_parser,
    )
        .any()
        .map(|res|match res {
            Ok(o)=>Ok(o),
            Err(e)=>Err(Error::Multiple(e)),
        });
    let stmt_parser=(
        &opt_nl_or_ws_parser,
        stmt_inner_parser,
        &nl_or_eof_parser,
    ).map(|res|match res {
        Ok((_,stmt,_))=>Ok(stmt),
        Err(e)=>Err(e),
    });
    let file_inner_parser=stmt_parser.repeated(0).map(|res|match res {
        Ok(stmts)=>Ok(Stmt::Block(stmts)),
        Err(e)=>Err(e),
    });
    let file_parser=(
        r"\A".into_parser(),
        file_inner_parser,
        r"\z".into_parser(),
    ).map(|res|match res {
        Ok((_,f,_))=>Ok(f),
        Err(e)=>Err(e),
    });
    let parser_create_end=parser_create_start.elapsed();
    println!("Took {:?} to create the parser",parser_create_end);
    let mut buffer=Buffer::new(&data);
    let start=Instant::now();
    dbg!(file_parser.parse(&mut buffer)).ok();
    let end=start.elapsed();
    println!("Parsed in {:?}",end);
}
