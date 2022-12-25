// use regex_syntax::ast::{
//     // parse::Parser,
//     // Ast,
//     Error as RegexError,
// };
use regex::{
    Regex,
    Error as RegexError,
};
use std::{
    fmt::{
        Debug,
        // Formatter,
        // Result as FmtResult,
    },
    collections::HashMap,
};


#[macro_export]
macro_rules! syntax {
    {type Out=$t:ty;$($other:tt)*}=>{
        {
            let mut pats=Patterns::<$t>::new();
            $crate::syntax_inner!(pats,$($other)*);
            pats
        }
    };
}
#[macro_export]
macro_rules! syntax_inner {
    ($pats:expr,$name:literal=>$first:literal$(|$extra:literal)+;$($end:tt)*)=>{
        $pats.choice($name,[$first $(,$extra)+]);
        $crate::syntax_inner!($pats,$($end)*);
    };
    ($pats:expr,$name:literal=>$first:literal$(,$extra:literal)* => $fold:expr;$($end:tt)*)=>{
        $pats.sequence($name,[$first $(,$extra)+],&$fold);
        $crate::syntax_inner!($pats,$($end)*);
    };
    ($pats:expr,$name:literal=>$first:literal* => $fold:expr;$($end:tt)*)=>{
        $pats.repeat($name,$first,&$fold);
        $crate::syntax_inner!($pats,$($end)*);
    };
    ($pats:expr,$name:literal=>$first:literal+ => $fold:expr;$($end:tt)*)=>{
        $pats.repeat1($name,$first,&$fold);
        $crate::syntax_inner!($pats,$($end)*);
    };
    ($pats:expr,$name:literal=>$regex:literal;$($end:tt)*)=>{
        $pats.terminal($name,$regex).unwrap();
        $crate::syntax_inner!($pats,$($end)*);
    };
    ($pats:expr,)=>{};
}


pub type FoldFunction<'a,T>=&'a dyn Fn(Vec<StrOrData<'a,T>>)->T;


enum Pattern<'a,'source,T> {
    Regex(Regex),
    Sequence {
        seq:Vec<&'a str>,
        fold:FoldFunction<'source,T>,
    },
    Choice(Vec<&'a str>),
    Repeat {
        pat:&'a str,
        fold:FoldFunction<'source,T>,
    },
    Repeat1 {
        pat:&'a str,
        fold:FoldFunction<'source,T>,
    },
}
// impl<'a,'source,T> Debug for Pattern<'a,'source,T> {
//     fn fmt(&self,f:&mut Formatter)->FmtResult {
//         match self {
//             Self::Regex(raw,_,_)=>write!(f,"r{:?}",raw),
//             Self::Sequence{seq,..}=>{
//                 write!(f,"Sequence(")?;
//                 seq.fmt(f)?;
//                 write!(f,")")
//             },
//             Self::Choice(cs)=>{
//                 write!(f,"Choice(")?;
//                 cs.fmt(f)?;
//                 write!(f,")")
//             },
//             _=>todo!(),
//         }
//     }
// }
impl<'a,'source,T> Pattern<'a,'source,T> {
    pub fn sequence<L:IntoIterator<Item=&'a str>>(items:L,fold:FoldFunction<'source,T>)->Self {
        return Self::Sequence{seq:items.into_iter().collect(),fold};
    }
    pub fn choice<L:IntoIterator<Item=&'a str>>(items:L)->Self {
        return Self::Choice(items.into_iter().collect());
    }
    pub fn repeat(pat:&'a str,fold:FoldFunction<'source,T>)->Self {
        return Self::Repeat{pat,fold};
    }
    pub fn repeat1(pat:&'a str,fold:FoldFunction<'source,T>)->Self {
        return Self::Repeat1{pat,fold};
    }
}
#[derive(Debug)]
pub enum StrOrData<'a,T> {
    Str(&'a str),
    Strings(Vec<&'a str>),
    Data(T),
}
impl<'a,T> StrOrData<'a,T> {
    pub fn is_data(&self)->bool {
        match self {
            Self::Data(_)=>true,
            _=>false,
        }
    }
    pub fn as_str(&self)->Option<&'a str> {
        match self {
            Self::Str(s)=>Some(s),
            _=>None,
        }
    }
    pub fn as_data(self)->Option<T> {
        match self {
            Self::Data(d)=>Some(d),
            _=>None,
        }
    }
}


pub struct Patterns<'a,'source,T> {
    patterns:HashMap<&'a str,Pattern<'a,'source,T>>,
}
impl<'a,'source,T:Debug> Patterns<'a,'source,T> {
    pub fn new()->Self {
        Self {
            patterns:HashMap::new(),
        }
    }
    pub fn terminal(&mut self,name:&'a str,regex:&'a str)->Result<(),RegexError> {
        let regex=format!("\\A{}",regex);
        // let ast=Parser::new().parse(&regex)?;
        // if it isn't valid, then skip the expensive regex compile
        let r=Regex::new(&regex).unwrap();
        self.patterns.insert(name,Pattern::Regex(r));
        return Ok(());
    }
    pub fn sequence<L:IntoIterator<Item=&'a str>>(&mut self,name:&'a str,patterns:L,fold:FoldFunction<'source,T>) {
        let seq=Pattern::sequence(patterns,fold);
        self.patterns.insert(name,seq);
    }
    pub fn choice<L:IntoIterator<Item=&'a str>>(&mut self,name:&'a str,patterns:L) {
        self.patterns.insert(name,Pattern::choice(patterns));
    }
    pub fn repeat(&mut self,name:&'a str,pat:&'a str,fold:FoldFunction<'source,T>) {
        let seq=Pattern::repeat(pat,fold);
        self.patterns.insert(name,seq);
    }
    pub fn repeat1(&mut self,name:&'a str,pat:&'a str,fold:FoldFunction<'source,T>) {
        let seq=Pattern::repeat1(pat,fold);
        self.patterns.insert(name,seq);
    }
    pub fn parse(&self,source:&'source str)->Option<StrOrData<'source,T>> {
        if let Some((_,d))=self.inner_parse("START",source) {
            return Some(d);
        }
        return None;
    }
    fn inner_parse(&self,name:&'a str,source:&'source str)->Option<(usize,StrOrData<'source,T>)> {
        // println!("Pattern `{}`",name);
        let pat=self.patterns.get(name).unwrap();
        match pat {
            Pattern::Regex(r)=>{
                let mut v=Vec::new();
                let caps=r.captures(source)?;
                let mut len=0;
                for (i,cap) in caps.iter().enumerate() {
                    if i==0 {
                        len=cap?.end();
                    }
                    v.push(cap?.as_str());
                }
                if v.len()==1 {
                    return Some((len,StrOrData::Str(v.remove(0))));
                }
                return Some((len,StrOrData::Strings(v)));
            },
            Pattern::Sequence{seq,fold}=>{
                let mut stack=Vec::new();
                let mut len=0;
                for name in seq.iter() {
                    let (add_len,item)=self.inner_parse(name,&source[len..])?;
                    len+=add_len;
                    stack.push(item);
                }
                return Some((len,StrOrData::Data(fold(stack))));
            },
            Pattern::Choice(choices)=>{
                for choice in choices {
                    let ret=self.inner_parse(choice,source);
                    if ret.is_some() {
                        return ret;
                    }
                }
                return None;
            },
            Pattern::Repeat{pat,fold}=>{
                let mut stack=Vec::new();
                let mut len=0;
                while let Some((add_len,item))=self.inner_parse(pat,&source[len..]) {
                    stack.push(item);
                    len+=add_len;
                }
                return Some((len,StrOrData::Data(fold(stack))));
            },
            Pattern::Repeat1{pat,fold}=>{
                let mut stack=Vec::new();
                let mut len=0;
                while let Some((add_len,item))=self.inner_parse(pat,&source[len..]) {
                    stack.push(item);
                    len+=add_len;
                }
                if stack.len()==0 {
                    return None;
                }
                return Some((len,StrOrData::Data(fold(stack))));
            },
        }
    }
}
