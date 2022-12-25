use std::fs::read_to_string;
use regex_parser::{
    Patterns,
    StrOrData,
    syntax,
};


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


fn main() {
    let data=read_to_string("example").unwrap();
    let pats=syntax!{
        type Out=Stmt;
        "START" => "file","EOF" => |mut items|items.remove(0).as_data().unwrap();
        "file" => "file inner"* => |items|{
            Stmt::Block(items.into_iter().filter_map(StrOrData::as_data).collect())
        };
        "file inner" => "stmt"|"NL"|"WS";
        "stmt" => "NL?","stmt inner","NL|EOF" => |mut items|items.remove(1).as_data().unwrap();
        "stmt inner" => "declare"|"assign";
        "declare" => "KW let","WS","WORD","WS?","EQUAL","WS?","NUMBER" =>|items|{
            let name=items[2].as_str().unwrap();
            let num=items[6].as_str().unwrap().parse().unwrap();
            return Stmt::Declare{name,num};
        };
        "assign" => "WORD","WS?","EQUAL","WS?","NUMBER" =>|items|{
            let name=items[0].as_str().unwrap();
            let num=items[4].as_str().unwrap().parse().unwrap();
            return Stmt::Assign{name,num};
        };
        "NL|EOF" => "NL"|"EOF";
        "WORD" => r"[a-zA-Z_][a-zA-Z0-9_]*";
        "NUMBER" => r"[0-9]*";
        "EQUAL" => "=";
        "WS" => "[ \t\r]+";
        "WS?" => "[ \t\r]*";
        "NL" => "\n+";
        "NL?" => "\n*";
        "EOF" => r"\z";
        "KW let" => "let";
    };
    println!("{:#?}",pats.parse(&data));
}
