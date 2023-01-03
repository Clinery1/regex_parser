use num_traits::Bounded;
use regex::{
    Regex,
    Captures,
    Error as RegexError,
};
use std::{
    rc::{
        Rc,
        Weak,
    },
    str::FromStr,
    marker::PhantomData,
    borrow::Borrow,
};


macro_rules! impl_tuple_parser {
    ($first_par_ty:ident$(,$par_ty:ident)*)=>{
        impl_tuple_parser!($first_par_ty,$($par_ty,)*);
    };
    ($($par_ty:ident,)+)=>{
        impl<'a,$($par_ty:Parser<'a,ERR>,)+ERR> Parser<'a,ERR> for ($($par_ty,)+) {
            type Output=($($par_ty::Output,)+);
            #[allow(non_snake_case)]
            fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,ERR> {
                let ($($par_ty,)+)=self;
                $(
                    let buf_start=buf.index;
                    let $par_ty=match $par_ty.parse(buf) {
                        Ok(t)=>t,
                        Err(e)=>{
                            buf.index=buf_start;
                            return Err(e.into());
                        },
                    };
                )+
                return Ok(($($par_ty,)+));
            }
        }
    };
}
macro_rules! impl_tuple_parser_ext3 {
    ($first_par_ty:ident$(,$par_ty:ident)*)=>{
        impl_tuple_parser_ext3!($first_par_ty,$($par_ty,)*);
    };
    ($($par_ty:ident,)+)=>{
        impl<'a,'b,OUT,$($par_ty:Parser<'a,ERR,Output=OUT>+'b,)+ERR> ParserExt3<'a,'b,ERR,OUT> for ($(&'b $par_ty,)+) {
            #[allow(non_snake_case)]
            fn any(self)->Choice<'a,'b,OUT,ERR> {
                let ($($par_ty,)+)=self;
                let group=[$(
                    $par_ty as &dyn Parser<'a,ERR,Output=OUT>,
                )+];
                return Choice::new(group);
            }
        }
    };
}


pub trait Parser<'a,E> {
    type Output;
    /// It is up to the implementer to make sure that `buf` is left in a reusable state upon error.
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E>;
}
impl<'a,E,P:Parser<'a,E>> Parser<'a,E> for Box<P> {
    type Output=P::Output;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E> {
        return <Box<P> as Borrow<P>>::borrow(self).parse(buf);
    }
}
impl<'a,E,P:Parser<'a,E>> Parser<'a,E> for Rc<P> {
    type Output=P::Output;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E> {
        return <Rc<P> as Borrow<P>>::borrow(self).parse(buf);
    }
}
impl<'a,E,P:Parser<'a,E>> Parser<'a,E> for Weak<P> {
    type Output=P::Output;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E> {
        let upgrade=self.upgrade().expect("Could not upgrade Weak<Parser> to Rc<Parser>");
        return <Rc<P> as Borrow<P>>::borrow(&upgrade).parse(buf);
    }
}
impl<'a,E,P:Parser<'a,E>> Parser<'a,E> for &P {
    type Output=P::Output;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E> {
        (*self).parse(buf)
    }
}
impl<'a,E,P:Parser<'a,E>> Parser<'a,E> for &mut P {
    type Output=P::Output;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E> {
        (**self).parse(buf)
    }
}
impl_tuple_parser!(A,B);
impl_tuple_parser!(A,B,C);
impl_tuple_parser!(A,B,C,D);
impl_tuple_parser!(A,B,C,D,E);
impl_tuple_parser!(A,B,C,D,E,F);
impl_tuple_parser!(A,B,C,D,E,F,G);
impl_tuple_parser!(A,B,C,D,E,F,G,H);
impl_tuple_parser!(A,B,C,D,E,F,G,H,I);
impl_tuple_parser!(A,B,C,D,E,F,G,H,I,J);

pub trait ParserExt<'a,'b,E>:Parser<'a,E>+Sized+'b {
    fn ignored(self)->Ignore<'a,E,Self> {
        Ignore::new(self)
    }
    fn map<T,E2,F:Fn(Result<Self::Output,E>)->Result<T,E2>>(self,closure:F)->Map<'a,T,E2,E,Self,F> {
        Map::new(self,closure)
    }
    fn boxed(self)->Box<dyn Parser<'a,E,Output=Self::Output>+'b> {
        let boxed=Box::new(self);
        return boxed;
    }
    fn repeated(self,times:usize)->Repeat<'a,Self::Output,E,Self> {
        return Repeat::new(self,times);
    }
}
impl<'a,'b,E,P:Parser<'a,E>+Sized+'b> ParserExt<'a,'b,E> for P {}

pub trait ParserExt3<'a,'b,E,T>:'b {
    fn any(self)->Choice<'a,'b,T,E>;
}
impl_tuple_parser_ext3!(A,B);
impl_tuple_parser_ext3!(A,B,C);
impl_tuple_parser_ext3!(A,B,C,D);
impl_tuple_parser_ext3!(A,B,C,D,E);
impl_tuple_parser_ext3!(A,B,C,D,E,F);
impl_tuple_parser_ext3!(A,B,C,D,E,F,G);
impl_tuple_parser_ext3!(A,B,C,D,E,F,G,H);
impl_tuple_parser_ext3!(A,B,C,D,E,F,G,H,I);
impl_tuple_parser_ext3!(A,B,C,D,E,F,G,H,I,J);

pub trait IntoRegexParser<E> {
    fn into_parser(&self)->RegexParser<E>;
}
impl<E> IntoRegexParser<E> for &str {
    fn into_parser(&self)->RegexParser<E> {
        RegexParser::new(self).expect("Invalid regex")
    }
}


enum RecursiveInner<T> {
    Owned(Rc<T>),
    Weak(Weak<T>),
}
impl<T> Clone for RecursiveInner<T> {
    fn clone(&self)->Self {
        match self {
            Self::Owned(rc)=>Self::Owned(rc.clone()),
            Self::Weak(weak)=>Self::Weak(weak.clone()),
        }
    }
}
impl<T> RecursiveInner<T> {
    fn get_rc(&self)->Rc<T> {
        match self {
            Self::Owned(rc)=>rc.clone(),
            Self::Weak(weak)=>weak.upgrade().unwrap(),
        }
    }
}


pub struct Buffer<'a> {
    raw:&'a str,
    index:usize,
}
impl<'a> Buffer<'a> {
    pub fn new(raw:&'a str)->Self {
        Buffer {
            raw,
            index:0,
        }
    }
    pub fn remaining(&self)->&'a str {
        &self.raw[self.index..]
    }
    pub fn add_offset(&mut self,to_add:usize) {
        self.index+=to_add;
    }
    pub fn try_match_regex(&self,regex:&Regex)->Option<Captures> {
        return regex.captures(self.remaining());
    }
    pub fn index(&self)->usize {self.index}
    pub fn set_index(&mut self,index:usize) {
        self.index=index;
    }
}

pub struct RegexParserError;
impl From<RegexParserError> for () {fn from(_:RegexParserError) {()}}

#[derive(Debug,Copy,Clone)]
pub struct RegexParserMatch<'a> {
    pub start:usize,
    pub end:usize,
    pub matched:&'a str,
}

/// Matches a regex. Results in a list of `RegexParserMatch`s with the first being the entire match
/// and each subsequent match being a capture group
pub struct RegexParser<E> {
    regex:Regex,
    _phantom:PhantomData<E>,
}
impl<E> Clone for RegexParser<E> {
    fn clone(&self)->Self {
        RegexParser {
            regex:self.regex.clone(),
            _phantom:PhantomData,
        }
    }
}
impl<E> RegexParser<E> {
    pub fn new(r:&str)->Result<Self,RegexError> {
        let regex=Regex::new(r)?;
        return Ok(RegexParser {
            regex,
            _phantom:PhantomData,
        });
    }
    fn parse_inner<'a>(&self,buf:&mut Buffer<'a>)->Option<Vec<RegexParserMatch<'a>>> {
        let mut out=Vec::new();
        let buf_offset=buf.index();
        let caps=self.regex.captures(buf.remaining())?;
        let mut captures=caps.iter().peekable();
        let first=captures.peek()?.expect("First regex capture should always exist");
        for cap in &mut captures {
            let cap=cap?;
            let end=cap.end()+buf_offset;
            let start=cap.start()+buf_offset;
            out.push(RegexParserMatch{start,end,matched:cap.as_str()});
        }
        buf.add_offset(first.end());
        return Some(out);
    }
}
impl<'a,E:From<RegexParserError>> Parser<'a,E> for RegexParser<E> {
    type Output=Vec<RegexParserMatch<'a>>;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Self::Output,E> {
        self.parse_inner(buf).ok_or(RegexParserError.into())
    }
}

pub struct Recursive<'a,T,E,P:Parser<'a,E,Output=T>>{
    inner:RecursiveInner<P>,
    _phantom:PhantomData<&'a (T,E)>,
}
impl<'a,T,E,P:Parser<'a,E,Output=T>> Clone for Recursive<'a,T,E,P> {
    fn clone(&self)->Self {
        Recursive {
            inner:self.inner.clone(),
            _phantom:PhantomData,
        }
    }
}
impl<'a,T,E,P:Parser<'a,E,Output=T>> Recursive<'a,T,E,P> {
    pub fn new<F:FnOnce(Self)->P>(closure:F)->Self {
        return Recursive {
            inner:RecursiveInner::Owned(Rc::new_cyclic(|weak|{
                let rec=Recursive {
                    inner:RecursiveInner::Weak(weak.clone()),
                    _phantom:PhantomData,
                };
                closure(rec)
            })),
            _phantom:PhantomData,
        };
    }
}
impl<'a,T,E,P:Parser<'a,E,Output=T>> Parser<'a,E> for Recursive<'a,T,E,P> {
    type Output=T;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<T,E> {
        return <Rc<P> as Borrow<P>>::borrow(&self.inner.get_rc()).parse(buf);
    }
}

pub struct Choice<'a,'b,T,E> {
    choices:Vec<&'b (dyn Parser<'a,E,Output=T>+'b)>,
}
impl<'a,'b,T,E> Choice<'a,'b,T,E> {
    pub fn new<I:IntoIterator<Item=&'b (dyn Parser<'a,E,Output=T>+'b)>>(parsers:I)->Self {
        Choice {
            choices:parsers.into_iter().collect(),
        }
    }
}
impl<'a,'b,T,E> Parser<'a,Vec<E>> for Choice<'a,'b,T,E> {
    type Output=T;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<T,Vec<E>> {
        let buf_index_start=buf.index;
        let mut errs=Vec::new();
        for choice in self.choices.iter() {
            match choice.parse(buf) {
                Ok(res)=>{
                    return Ok(res);
                },
                Err(e)=>{
                    errs.push(e);
                    buf.index=buf_index_start;
                },
            }
        }
        return Err(errs);
    }
}

pub struct Repeat<'a,T,E,P:Parser<'a,E,Output=T>> {
    inner:P,
    min_count:usize,
    _phantom:PhantomData<&'a (T,E)>,
}
impl<'a,T,E,P:Parser<'a,E,Output=T>> Repeat<'a,T,E,P> {
    pub fn new(parser:P,min_count:usize)->Self {
        Repeat {
            inner:parser,
            min_count,
            _phantom:PhantomData,
        }
    }
}
impl<'a,T,E,P:Parser<'a,E,Output=T>> Parser<'a,E> for Repeat<'a,T,E,P> {
    type Output=Vec<T>;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<Vec<T>,E> {
        let mut out=Vec::new();
        let buf_start_outer=buf.index;
        loop {
            let buf_start=buf.index;
            match self.inner.parse(buf) {
                Ok(t)=>out.push(t),
                Err(e)=>if out.len()<self.min_count {
                    buf.index=buf_start_outer;
                    return Err(e);
                } else {
                    buf.index=buf_start;
                    break;
                },
            }
        }
        return Ok(out);
    }
}

pub struct Map<'a,T,E,E2,P:Parser<'a,E2>,F:Fn(Result<P::Output,E2>)->Result<T,E>> {
    prev:P,
    closure:F,
    _phantom:PhantomData<&'a (T,E,E2)>,
}
impl<'a,T,E,E2,P:Parser<'a,E2>,F:Fn(Result<P::Output,E2>)->Result<T,E>> Map<'a,T,E,E2,P,F> {
    pub fn new(prev:P,closure:F)->Self {
        Map {
            prev,
            closure,
            _phantom:PhantomData,
        }
    }
}
impl<'a,T,E,E2,P:Parser<'a,E2>,F:Fn(Result<P::Output,E2>)->Result<T,E>> Parser<'a,E> for Map<'a,T,E,E2,P,F> {
    type Output=T;
    fn parse(&self,buf:&mut Buffer<'a>)->Result<T,E> {
        let buf_start=buf.index;
        let out=self.prev.parse(buf);
        if out.is_err() {
            buf.index=buf_start;
        }
        return (self.closure)(out);
    }
}

pub struct Ignore<'a,E,P:Parser<'a,E>> {
    inner:P,
    _phantom:PhantomData<&'a E>,
}
impl<'a,E,P:Parser<'a,E>> Ignore<'a,E,P> {
    pub fn new(parser:P)->Self {
        Ignore {
            inner:parser,
            _phantom:PhantomData,
        }
    }
}
impl<'a,E,P:Parser<'a,E>> Parser<'a,E> for Ignore<'a,E,P> {
    type Output=();
    fn parse(&self,buf:&mut Buffer<'a>)->Result<(),E> {
        match self.inner.parse(buf) {
            Ok(_)=>Ok(()),
            Err(e)=>return Err(e),
        }
    }
}


pub fn number_parser<'a,NUM:Bounded+FromStr+'a,E:From<RegexParserError>+From<<NUM as FromStr>::Err>+'a>()->impl Parser<'a,E,Output=NUM> {
    return Map::new(
        RegexParser::new(r"[0-9]*").unwrap(),
        |number_res:Result<_,E>|match number_res {
            Ok(matches)=>{
                match matches[0].matched.parse::<NUM>() {
                    Ok(val)=>Ok(val),
                    Err(e)=>Err(e.into()),
                }
            },
            Err(e)=>Err(e.into()),
        },
    );
}
