// A Parser for Things
// is a function from Strings
// to Lists of Pairs
// of Things and Strings
// https://people.willamette.edu/~fruehr/haskell/seuss.html
//
// More specifically in our case to a Result of the
// remaining string and the Thing, or an error. We
// do this because while grammars _can_ be
// non-deterministic in their parse tree, this often
// isn't the case, nor helpful. An error telling us
// what happened is more useful in most cases.

use std::{collections::LinkedList, fmt::Debug, marker::PhantomData, process::Output, str::Chars};

pub trait Input: Clone + Sized {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn iter(i: &Self) -> Self::Iter;
    fn from_iter(i: &Self::Iter) -> Self;
}
impl<'s> Input for &'s str {
    type Item = char;
    type Iter = Chars<'s>;

    fn iter(i: &&'s str) -> Self::Iter {
        i.chars() as Chars<'s>
    }
    fn from_iter(i: &Chars<'s>) -> Self {
        i.as_str()
    }
}

pub type ParseResult<I, O, E> = Result<(I, O), E>;

#[derive(Clone)]
pub struct Ap<PF, PA> {
    pub pf: PF,
    pub pa: PA,
}
impl<I, O, E, A, F, PA, PF> Parser<I> for Ap<PF, PA>
where
    I: Input,
    F: Fn(&A) -> O,
    PF: Parser<I, Output = F, Error = E>,
    PA: Parser<I, Output = A, Error = E>,
{
    type Output = O;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, f) = self.pf.parse(i)?;
        let (i, a) = self.pa.parse(&i)?;

        Ok((i, f(&a)))
    }
}

pub fn ap<I: Input, A, O, F, PF, PA, E>(pf: PF, pa: PA) -> impl Parser<I, Output = O, Error = E>
where
    F: Fn(&A) -> O,
    PF: Parser<I, Output = F, Error = E>,
    PA: Parser<I, Output = A, Error = E>,
{
    pf.ap::<A, O, F, PF, PA, E>(pa)
}

#[derive(Clone)]
pub struct Map<P, F> {
    pub parser: P,
    pub func: F,
}
impl<I, O, E, P, F> Parser<I> for Map<P, F>
where
    I: Input,
    P: Parser<I, Error = E>,
    F: Fn(&P::Output) -> O + Clone,
{
    type Output = O;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self.parser.parse(i) {
            Err(e) => Err(e),
            Ok((i, o)) => Ok((i, (self.func)(&o))),
        }
    }
}

pub fn map<I: Input, O, E, P, F>(parser: P, f: F) -> impl Parser<I, Output = O, Error = E>
where
    P: Parser<I, Error = E>,
    F: Fn(&P::Output) -> O + Clone,
{
    parser.map(f)
}

#[derive(Clone)]
pub struct Some<P> {
    pub parser: P,
}
impl<I, O, E, P> Parser<I> for Some<P>
where
    I: Input,
    P: Parser<I, Output = O, Error = E>,
{
    type Output = LinkedList<P::Output>;
    type Error = P::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let parser = self.parser.clone();
        match self.parser.parse(i) {
            Ok((i, o)) => many(parser).parse(&i).map(|(i, mut oo)| {
                oo.push_front(o);
                (i, oo)
            }),
            Err(e) => Err(e),
        }
    }
}

pub fn some<I: Input, O, E, P>(p: P) -> Some<P>
where
    P: Parser<I, Output = O, Error = E>,
{
    p.some()
}

#[derive(Clone)]
pub struct Many<P> {
    pub parser: P,
}
impl<I, O, E, P> Parser<I> for Many<P>
where
    I: Input,
    P: Parser<I, Output = O, Error = E>,
{
    type Output = LinkedList<P::Output>;
    type Error = P::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let parser = self.parser.clone();
        match self.parser.parse(i) {
            Ok((i, o)) => many(parser).parse(&i).map(|(i, mut oo)| {
                oo.push_front(o);
                (i, oo)
            }),
            Err(_) => Ok((i.to_owned(), LinkedList::new())),
        }
    }
}

pub fn many<I: Input, O, E, P>(p: P) -> Many<P>
where
    P: Parser<I, Output = O, Error = E>,
{
    p.many()
}

#[derive(Clone)]
pub struct Pure<I, O, E> {
    value: O,
    phantom: PhantomData<(I, E)>,
}
impl<I, O, E> Parser<I> for Pure<I, O, E>
where
    I: Input,
    O: Clone,
    E: Clone,
{
    type Output = O;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        Ok((i.to_owned(), self.value.clone()))
    }
}

pub fn pure<I: Input, O, E>(o: O) -> Pure<I, O, E> {
    Pure {
        value: o,
        phantom: PhantomData,
    }
}

#[derive(Clone)]
pub struct Satisfy<P, F> {
    parser: P,
    f: F,
}
impl<I, P, F> Parser<I> for Satisfy<P, F>
where
    I: Input,
    P: Parser<I>,
    F: Fn(&P::Output) -> bool + Clone,
    P::Error: From<String>,
    P::Output: Debug,
{
    type Output = P::Output;
    type Error = P::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (input, o) = self.parser.parse(i)?;
        if (self.f)(&o) {
            Ok((input, o))
        } else {
            Result::Err(format!("Unexpected input: {:?}", o).into())
        }
    }
}

#[derive(Clone)]
pub struct Surrounded<P, A> {
    parser: P,
    surround: A,
}
impl<I: Input, E, P: Parser<I, Error = E>, A: Parser<I, Error = E>> Parser<I> for Surrounded<P, A> {
    type Output = P::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, _) = self.surround.parse(&i)?;
        let (i, o) = self.parser.parse(&i)?;
        let (i, _) = self.surround.parse(&i)?;

        Ok((i, o))
    }
}

pub fn surround<I: Input, O, SO, E, P, S>(parser: P, surround: S) -> Surrounded<P, S>
where
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Output = SO, Error = E>,
{
    parser.surround(surround)
}

#[derive(Clone)]
pub struct Bracket<P, B, K> {
    parser: P,
    brac: B,
    ket: K,
}
impl<I: Input, E, P: Parser<I, Error = E>, B: Parser<I, Error = E>, K: Parser<I, Error = E>>
    Parser<I> for Bracket<P, B, K>
{
    type Output = P::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, _) = self.brac.parse(&i)?;
        let (i, o) = self.parser.parse(&i)?;
        let (i, _) = self.ket.parse(&i)?;

        Ok((i, o))
    }
}

pub fn bracket<I: Input, O, BO, KO, E, P, B, K>(parser: P, brac: B, ket: K) -> Bracket<P, B, K>
where
    P: Parser<I, Output = O, Error = E>,
    B: Parser<I, Output = BO, Error = E>,
    K: Parser<I, Output = KO, Error = E>,
{
    parser.bracket(brac, ket)
}

#[derive(Clone)]
pub struct Optional<P> {
    parser: P,
}
impl<I: Input, E, P: Parser<I, Error = E>> Parser<I> for Optional<P> {
    type Output = Option<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self.parser.parse(&i) {
            Err(_) => Ok((i.to_owned(), None)),
            Ok((i, o)) => Ok((i, Some(o))),
        }
    }
}

pub fn optional<I: Input, O, E, P>(parser: P) -> Optional<P>
where
    P: Parser<I, Output = O, Error = E>,
{
    parser.optional()
}

#[derive(Clone)]
pub struct SkipOptional<P> {
    parser: P,
}
impl<I: Input, E, P: Parser<I, Error = E>> Parser<I> for SkipOptional<P> {
    type Output = ();
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self.parser.parse(&i) {
            Err(_) => Ok((i.to_owned(), ())),
            Ok((i, _)) => Ok((i, ())),
        }
    }
}

pub fn skip_optional<I: Input, O, E, P>(parser: P) -> SkipOptional<P>
where
    P: Parser<I, Output = O, Error = E>,
{
    parser.skip_optional()
}

#[derive(Clone)]
pub struct Count<P> {
    parser: P,
    count: u32
}
impl<I: Input, E, P: Parser<I, Error = E>> Parser<I> for Count<P> {
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let parser = self.parser.clone();
        match self.count {
            0 => Ok((i.to_owned(), LinkedList::new())),
            n => {
                match self.parser.parse(i) {
                    Err(e) => Err(e),
                    Ok((i, o)) => match parser.count(n-1).parse(&i) {
                        Err(e) => Err(e),
                        Ok((i, mut l)) => {
                            l.push_front(o);
                            Ok((i, l))
                        }
                    }
                }
            }
        }
    }
}

pub fn count<I: Input, O, E, P>(parser: P, n: u32) -> Count<P>
where
    P: Parser<I, Output = O, Error = E>,
{
    parser.count(n)
}

#[derive(Clone)]
pub struct ManyTill<P, Q> {
    parser: P,
    end: Q
}
impl<I: Input, E, P: Parser<I, Error = E>, Q: Parser<I, Error=E>> Parser<I> for ManyTill<P, Q> {
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let parser = self.parser.clone();
        let end = self.end.clone();

        match self.end.parse(i) {
            Ok((i, _)) => Ok((i, LinkedList::new())),
            Err(_) => match self.parser.parse(i) {
                Err(e) => Err(e),
                Ok((i, o)) => {
                    match many_till(parser, end).parse(&i) {
                        Ok((i, mut l)) => {
                            l.push_front(o);
                            Ok((i, l))
                        },
                        Err(e) => Err(e)
                    }
                }
            }
        }
    }
}

pub fn many_till<I: Input, O, E, P, Q>(parser: P, end: Q) -> ManyTill<P, Q>
where
    P: Parser<I, Output = O, Error = E>,
    Q: Parser<I, Error = E>,
{
    parser.many_till(end)
}

#[derive(Clone)]
pub struct SepBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, E, P: Parser<I, Error = E>, S: Parser<I, Error = E>> Parser<I> for SepBy<P, S> {
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let sep = self.sep.clone();
        let parser = self.parser.clone();
        match self.parser.clone().parse(i) {
            Ok((i, o)) => many(sep.seq(parser)).parse(&i).map(|(i, mut l)| {
                l.push_front(o);
                (i.to_owned(), l)
            }),
            _ => Ok((i.to_owned(), LinkedList::new())),
        }
    }
}

pub fn sep_by<I: Input, O, E, P, S>(
    parser: P,
    sep: S,
) -> impl Parser<I, Output = LinkedList<O>, Error = E>
where
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
{
    parser.sep_by(sep)
}

#[derive(Clone)]
pub struct SepBy1<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, E, P: Parser<I, Error = E>, S: Parser<I, Error = E>> Parser<I> for SepBy1<P, S> {
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let sep = self.sep.clone();
        let parser = self.parser.clone();
        match self.parser.clone().parse(i) {
            Ok((i, o)) => many(sep.seq(parser)).parse(&i).map(|(i, mut l)| {
                l.push_front(o);
                (i.to_owned(), l)
            }),
            Err(e) => Err(e),
        }
    }
}

pub fn sep_by_1<I: Input, O, E, P, S>(
    parser: P,
    sep: S,
) -> impl Parser<I, Output = LinkedList<O>, Error = E>
where
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
{
    parser.sep_by_1(sep)
}

#[derive(Clone)]
pub struct Seq<P, Q> {
    a: P,
    b: Q,
}

impl<I: Input, E, P: Parser<I, Error = E>, Q: Parser<I, Error = E>> Parser<I> for Seq<P, Q> {
    type Output = Q::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, _) = self.a.parse(i)?;
        self.b.parse(&i)
    }
}

pub fn seq<I: Input, O, E, P, Q>(a: P, b: Q) -> impl Parser<I, Output = Q::Output, Error = E>
where
    P: Parser<I, Error = E>,
    Q: Parser<I, Output = O, Error = E>,
{
    a.seq(b)
}

#[derive(Clone)]
pub struct And<F, G> {
    f: F,
    g: G,
}

pub fn and<I: Input, A, B, E, F, G>(f: F, g: G) -> impl Parser<I, Output = (A, B), Error = E>
where
    F: Parser<I, Output = A, Error = E>,
    G: Parser<I, Output = B, Error = E>,
{
    f.and(g)
}

impl<I: Input, E, F: Parser<I, Error = E>, G: Parser<I, Error = E>> Parser<I> for And<F, G> {
    type Output = (F::Output, G::Output);
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, a) = self.f.parse(i)?;
        let (i, b) = self.g.parse(&i)?;

        Ok((i, (a, b)))
    }
}

#[derive(Clone)]
pub struct AndThen<P, F> {
    parser: P,
    f: F,
}
impl<
    I: Input,
    E,
    A,
    B,
    P: Parser<I, Output = A, Error = E>,
    Q: Parser<I, Output = B, Error = E>,
    F: Fn(&A) -> Q + Clone,
> Parser<I> for AndThen<P, F>
{
    type Output = Q::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, a) = self.parser.parse(i)?;
        let (i, b) = (self.f)(&a).parse(&i)?;
        Ok((i, b))
    }
}

pub fn and_then<
    I: Input,
    E,
    A,
    B,
    P: Parser<I, Output = A, Error = E>,
    Q: Parser<I, Output = B, Error = E>,
    F: Fn(&A) -> Q + Clone,
>(
    parser: P,
    func: F,
) -> impl Parser<I, Output = B, Error = E> {
    parser.and_then(func)
}

#[derive(Clone)]
pub struct Or<F, G> {
    f: F,
    g: G,
}
impl<I: Input, O, E, F: Parser<I, Output = O, Error = E>, G: Parser<I, Output = O, Error = E>>
    Parser<I> for Or<F, G>
{
    type Output = F::Output;
    type Error = F::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self.f.parse(i) {
            Err(_e1) => match self.g.parse(i) {
                Err(e2) => Err(e2),
                res => res,
            },
            res => res,
        }
    }
}

pub fn or<I: Input, O, E, F, G>(f: F, g: G) -> impl Parser<I, Output = O, Error = E>
where
    F: Parser<I, Output = O, Error = E>,
    G: Parser<I, Output = O, Error = E>,
{
    f.or(g)
}

pub trait Parser<I: Input>: Clone + Sized {
    type Output;
    type Error;

    // Required method
    fn parse(&self, input: &I) -> ParseResult<I, Self::Output, Self::Error>;

    // Provided methods
    fn map<F, O2>(self, f: F) -> Map<Self, F>
    where
        F: Fn(&Self::Output) -> O2,
        Self: Sized,
    {
        Map {
            parser: self,
            func: f,
        }
    }
    fn and<G, O>(self, g: G) -> And<Self, G>
    where
        G: Parser<I, Output = O, Error = Self::Error>,
        Self: Sized,
    {
        And { f: self, g: g }
    }
    fn and_then<F>(self, f: F) -> AndThen<Self, F>
    where
        Self: Sized,
    {
        AndThen { parser: self, f: f }
    }

    fn ap<A, O, F, PF, PA, E>(self, pa: PA) -> Ap<Self, PA>
    where
        F: Fn(&A) -> O,
        PA: Parser<I, Output = A, Error = E>,
        Self: Sized + Parser<I, Output = F, Error = E>,
    {
        Ap { pf: self, pa: pa }
    }

    fn or<G>(self, g: G) -> Or<Self, G>
    where
        G: Parser<I, Output = Self::Output, Error = Self::Error>,
        Self: Sized,
    {
        Or { f: self, g: g }
    }
    fn satisfy<F>(self, f: F) -> Satisfy<Self, F>
    where
        F: Fn(&Self::Output) -> bool,
        Self: Sized,
    {
        Satisfy { parser: self, f: f }
    }
    fn many(self) -> Many<Self>
    where
        Self: Sized,
    {
        Many { parser: self }
    }
    fn some(self) -> Some<Self>
    where
        Self: Sized,
    {
        Some { parser: self }
    }
    fn surround<S>(self, surround: S) -> Surrounded<Self, S> {
        Surrounded {
            parser: self,
            surround,
        }
    }
    fn bracket<B, K>(self, brac: B, ket: K) -> Bracket<Self, B, K> {
        Bracket {
            parser: self,
            brac,
            ket,
        }
    }
    fn optional(self) -> Optional<Self> {
        Optional { parser: self }
    }
    fn skip_optional(self) -> SkipOptional<Self> {
        SkipOptional { parser: self }
    }
    fn seq<Q>(self, b: Q) -> Seq<Self, Q> {
        Seq { a: self, b }
    }
    fn sep_by<S>(self, sep: S) -> SepBy<Self, S> {
        SepBy { parser: self, sep }
    }
    fn sep_by_1<S>(self, sep: S) -> SepBy1<Self, S> {
        SepBy1 { parser: self, sep }
    }
    fn count(self, n: u32) -> Count<Self> {
        Count { parser: self, count: n }
    }
    fn many_till<Q>(self, end: Q) -> ManyTill<Self, Q> {
        ManyTill { parser: self, end }
    }
}

#[derive(Clone)]
pub struct AnyChar();
impl<I: Input<Item = char>> Parser<I> for AnyChar {
    type Output = I::Item;
    type Error = String;

    fn parse(&self, input: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let mut i = I::iter(input);
        match i.next() {
            None => Result::Err("End Of File".to_string()),
            Some(c) => Result::Ok((I::from_iter(&i), c)),
        }
    }
}

#[derive(Clone)]
pub struct EOF();
impl<I: Input<Item : Debug>> Parser<I> for EOF where {
    type Output = ();
    type Error = String;

    fn parse(&self, input: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let mut i = I::iter(input);
        match i.next() {
            None => Ok((input.to_owned(), ())),
            Some(c) => Err(format!("Expected EOF, got {:?}", c))
        }
    }
}

pub fn char<I: Input<Item = char>>(c: char) -> impl Parser<I, Output = char, Error = String> {
    <AnyChar as Parser<I>>::satisfy::<_>(AnyChar(), move |c_| c_ == &c)
}

pub fn eof<I: Input<Item: Debug>>() -> impl Parser<I, Output = (), Error = String> {
    EOF()
}

pub fn foo<I: Input<Item = char>>() -> impl Parser<I, Output = String, Error = String> {
    and_then(
        and(
            map(
                or(
                    char('a').and(char('b')),
                    char('c').and(char('d')),
                ),
                |t| format!("({}. {})", t.0, t.1),
            ),
            ap(
                pure(|l: &LinkedList<Option<char>>| format!("{:?}", l)),
                some(bracket(optional(surround(char('e'), char('\''))), char('['), char(']'))),
            ),
        ),
        |t: &(String, String)| pure(format!("{:?} and {:?}", t.0, t.1)),
    ).and_then(|s: &String| {
        let s = s.to_owned();
        count(char('f'), 3).map(move |_| s.clone())
    }).and_then(|s: &String| {
        let s = s.to_owned();
        many_till(char('g'), eof()).map(move |_| s.clone())
    })
}