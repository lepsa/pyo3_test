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
use std::{fmt::Debug, marker::PhantomData, str::Chars, vec::Vec};

use pyo3::{exceptions::PyException, *};

#[derive(Clone, Debug)]
#[pyclass]
pub enum ParseErr {
    EOF(),
    Unexpected(String),
    Expected(char),
}

impl ParseErr {
    pub fn to_string(&self) -> String {
        match self {
            ParseErr::EOF() => "End Of File".to_string(),
            ParseErr::Unexpected(msg) => format!("Unexpected: {}", msg),
            ParseErr::Expected(c) => format!("Expected to see char: {}", c),
        }
    }
}
impl Into<PyErr> for ParseErr {
    fn into(self) -> PyErr {
        match self {
            ParseErr::EOF() => PyErr::new::<PyException, _>("EOF".to_string()),
            ParseErr::Unexpected(msg) => {
                PyErr::new::<PyException, _>(format!("Unexpected: {}", msg))
            }
            ParseErr::Expected(c) => PyErr::new::<PyException, _>(format!("Expected: '{}'", c)),
        }
    }
}

pub trait Input: Clone {
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

pub type ParseResult<I, O> = Result<(I, O), ParseErr>;

#[derive(Clone)]
pub struct Ap<PF, PA> {
    pub pf: PF,
    pub pa: PA,
}
impl<I: Input, O, A, F: Fn(A) -> O, PA: Parser<I, Output = A>, PF: Parser<I, Output = F>> Parser<I>
    for Ap<PF, PA>
{
    type Output = O;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, f) = self.pf.parse(i)?;
        let (i, a) = self.pa.parse(i)?;
        Ok((i, f(a)))
    }
}

#[derive(Clone)]
pub struct Map<P, F> {
    pub parser: P,
    pub func: F,
}
impl<I: Input, O, P: Parser<I>, F: Fn(P::Output) -> O + Clone> Parser<I> for Map<P, F> {
    type Output = O;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, o) = self.parser.parse(i)?;
        Ok((i, (self.func)(o)))
    }
}

#[derive(Clone)]
pub struct Some<P> {
    pub parser: P,
}
impl<I: Input, O, P: Parser<I, Output = O>> Parser<I> for Some<P> {
    type Output = Vec<O>;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        let (mut i, o) = self.parser.parse(i)?;
        l.push(o);
        while let Ok((ii, o)) = self.parser.parse(i.clone()) {
            i = ii;
            l.push(o);
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct Many<P> {
    pub parser: P,
}
impl<I: Input, O, P: Parser<I, Output = O>> Parser<I> for Many<P> {
    type Output = Vec<O>;

    fn parse(&self, mut i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        while let Ok((ii, o)) = self.parser.parse(i.clone()) {
            l.push(o);
            i = ii;
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct Lift<I, F> {
    func: F,
    phantom: PhantomData<I>,
}
impl<I: Input, O, F: Fn(I) -> ParseResult<I, O> + Clone> Parser<I> for Lift<I, F> {
    type Output = O;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        (self.func)(i)
    }
}

pub fn lift<I: Input, O, F: Fn(I) -> ParseResult<I, O>>(func: F) -> Lift<I, F> {
    Lift {
        func,
        phantom: PhantomData,
    }
}

#[derive(Clone)]
pub struct Pure<I, F> {
    f: F,
    phantom: PhantomData<I>,
}
impl<A, F: Fn() -> A + Clone, I: Input> Parser<I> for Pure<I, F> {
    type Output = A;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        Ok((i, (self.f)()))
    }
}

pub fn pure<I: Input, O, F: Fn() -> O>(f: F) -> Pure<I, F> {
    Pure {
        f,
        phantom: PhantomData,
    }
}

#[derive(Clone)]
pub struct Satisfy<P, F> {
    parser: P,
    f: F,
}
impl<I: Input, P: Parser<I, Output: Debug>, F: Fn(&P::Output) -> bool + Clone> Parser<I>
    for Satisfy<P, F>
{
    type Output = P::Output;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (input, o) = self.parser.parse(i)?;
        if (self.f)(&o) {
            Ok((input, o))
        } else {
            Result::Err(ParseErr::Unexpected(format!("Unexpected input: {:?}", o)))
        }
    }
}

pub fn satisfy<I: Input, P: Parser<I, Output: Debug>, F: Fn(&P::Output) -> bool>(
    parser: P,
    f: F,
) -> Satisfy<P, F> {
    parser.satisfy(f)
}

#[derive(Clone)]
pub struct Surrounded<P, A> {
    parser: P,
    surround: A,
}
impl<I: Input, P: Parser<I>, A: Parser<I>> Parser<I> for Surrounded<P, A> {
    type Output = P::Output;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.surround.parse(i)?;
        let (i, o) = self.parser.parse(i)?;
        let (i, _) = self.surround.parse(i)?;
        Ok((i, o))
    }
}

#[derive(Clone)]
pub struct Bracket<P, B, K> {
    parser: P,
    brac: B,
    ket: K,
}
impl<I: Input, P: Parser<I>, B: Parser<I>, K: Parser<I>> Parser<I> for Bracket<P, B, K> {
    type Output = P::Output;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.brac.parse(i)?;
        let (i, o) = self.parser.parse(i)?;
        let (i, _) = self.ket.parse(i)?;
        Ok((i, o))
    }
}

#[derive(Clone)]
pub struct Optional<P> {
    parser: P,
}
impl<I: Input, P: Parser<I>> Parser<I> for Optional<P> {
    type Output = Option<P::Output>;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        match self.parser.parse(i.clone()) {
            Err(_) => Ok((i, None)),
            Ok((i, o)) => Ok((i, Some(o))),
        }
    }
}

#[derive(Clone)]
pub struct Void<P> {
    parser: P,
}
impl<I: Input, P: Parser<I>> Parser<I> for Void<P> {
    type Output = ();

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.parser.parse(i)?;
        Ok((i, ()))
    }
}

#[derive(Clone)]
pub struct SkipOptional<P> {
    parser: P,
}
impl<I: Input, P: Parser<I>> Parser<I> for SkipOptional<P> {
    type Output = ();

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        match self.parser.parse(i.clone()) {
            Err(_) => Ok((i, ())),
            Ok((i, _)) => Ok((i, ())),
        }
    }
}

#[derive(Clone)]
pub struct SkipMany<P> {
    parser: P,
}
impl<I: Input, P: Parser<I>> Parser<I> for SkipMany<P> {
    type Output = ();

    fn parse(&self, mut i: I) -> ParseResult<I, Self::Output> {
        while let Ok((ii, _)) = self.parser.parse(i.clone()) {
            i = ii;
        }
        Ok((i, ()))
    }
}

#[derive(Clone)]
pub struct SkipSome<P> {
    parser: P,
}
impl<I: Input, P: Parser<I>> Parser<I> for SkipSome<P> {
    type Output = ();

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (mut i, _) = self.parser.parse(i)?;
        while let Ok((ii, _)) = self.parser.parse(i.clone()) {
            i = ii;
        }
        Ok((i, ()))
    }
}

#[derive(Clone)]
pub struct NotFollowedBy<P, Q> {
    parser: P,
    follow: Q,
}
impl<I: Input, O, P: Parser<I, Output = O>, Q: Parser<I, Output: Debug>> Parser<I>
    for NotFollowedBy<P, Q>
{
    type Output = P::Output;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, o) = self.parser.parse(i)?;
        match self.follow.parse(i.clone()) {
            Err(_) => Ok((i, o)),
            Ok((_, e)) => Err(ParseErr::Unexpected(format!("Unexpected: {:?}", e))),
        }
    }
}

#[derive(Clone)]
pub struct Unexpected<I, O> {
    message: String,
    phantom: PhantomData<(I, O)>,
}
impl<I: Input, O: Clone> Parser<I> for Unexpected<I, O> {
    type Output = O;

    fn parse(&self, _i: I) -> ParseResult<I, Self::Output> {
        Err(ParseErr::Unexpected(self.message.clone()))
    }
}

pub fn unexpected<I: Input, O>(message: String) -> Unexpected<I, O> {
    Unexpected {
        message,
        phantom: PhantomData,
    }
}

#[derive(Clone)]
pub struct Count<P> {
    parser: P,
    count: u32,
}
impl<'p, I: Input, O, P: Parser<I, Output = O>> Parser<I> for Count<P> {
    type Output = Vec<O>;

    fn parse(&self, mut i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        for _ in 0..self.count {
            let (ii, o) = self.parser.parse(i)?;
            i = ii;
            l.push(o);
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct ManyTill<P, Q> {
    parser: P,
    end: Q,
}
impl<I: Input, O, P: Parser<I, Output = O>, Q: Parser<I>> Parser<I> for ManyTill<P, Q> {
    type Output = Vec<O>;

    fn parse(&self, mut i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        while let Err(_) = self.end.parse(i.clone()) {
            match self.parser.parse(i) {
                Ok((ii, a)) => {
                    l.push(a);
                    i = ii;
                }
                Err(e) => return Err(e),
            }
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct SepBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O, P: Parser<I, Output = O>, S: Parser<I>> Parser<I> for SepBy<P, S> {
    type Output = Vec<O>;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        match self.parser.parse(i.clone()) {
            Err(_) => Ok((i, l)),
            Ok((mut i, o)) => {
                l.push(o);
                while let Ok((ii, _)) = self.sep.parse(i.clone()) {
                    let (ii, o) = self.parser.parse(ii)?;
                    l.push(o);
                    i = ii;
                }
                Ok((i, l))
            }
        }
    }
}

#[derive(Clone)]
pub struct EndBy1<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O, SO, P: Parser<I, Output = O>, S: Parser<I, Output = SO>> Parser<I>
    for EndBy1<P, S>
{
    type Output = Vec<O>;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        let (i, o) = self.parser.parse(i)?;
        l.push(o);
        let (mut i, _) = self.sep.parse(i)?;
        while let Ok((ii, o)) = self.parser.parse(i.clone()) {
            let (ii, _) = self.sep.parse(ii)?;
            i = ii;
            l.push(o);
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct EndBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O, P: Parser<I, Output = O>, S: Parser<I>> Parser<I> for EndBy<P, S> {
    type Output = Vec<O>;

    fn parse(&self, mut i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        while let Ok((ii, o)) = self.parser.parse(i.clone()) {
            let (ii, _) = self.sep.parse(ii)?;
            i = ii;
            l.push(o);
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct SepEndBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O, P: Parser<I, Output = O>, S: Parser<I>> Parser<I> for SepEndBy<P, S> {
    type Output = Vec<O>;

    fn parse(&self, mut i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        while let Ok((ii, o)) = self.parser.parse(i.clone()) {
            match self.sep.parse(ii.clone()) {
                Ok((ii, _)) => {
                    i = ii;
                    l.push(o);
                    // Keep on looping!
                }
                Err(_) => {
                    i = ii;
                    l.push(o);
                    break;
                }
            }
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct SepEndBy1<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, A, B, P: Parser<I, Output = A>, S: Parser<I, Output = B>> Parser<I>
    for SepEndBy1<P, S>
{
    type Output = Vec<A>;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();

        let (mut i, o) = self.parser.parse(i)?;
        l.push(o);
        match self.sep.parse(i.clone()) {
            Ok((ii, _)) => {
                i = ii;
                // Go to the loop
            }
            Err(_) => {
                // Do not go into the loop
                return Ok((i, l));
            }
        }
        while let Ok((ii, o)) = self.parser.parse(i.clone()) {
            match self.sep.parse(ii.clone()) {
                Ok((ii, _)) => {
                    i = ii;
                    l.push(o);
                    // Keep on looping!
                }
                Err(_) => {
                    i = ii;
                    l.push(o);
                    break;
                }
            }
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct SepBy1<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O, SO, P: Parser<I, Output = O>, S: Parser<I, Output = SO>> Parser<I>
    for SepBy1<P, S>
{
    type Output = Vec<O>;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let mut l = Vec::new();
        let (mut i, o) = self.parser.parse(i)?;
        l.push(o);
        while let Ok((ii, _)) = self.sep.parse(i.clone()) {
            let (ii, o) = self.parser.parse(ii)?;
            l.push(o);
            i = ii;
        }
        Ok((i, l))
    }
}

#[derive(Clone)]
pub struct Seq<P, Q> {
    a: P,
    b: Q,
}

impl<I: Input, P: Parser<I>, Q: Parser<I>> Parser<I> for Seq<P, Q> {
    type Output = Q::Output;

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.a.parse(i)?;
        self.b.parse(i)
    }
}

#[derive(Clone)]
pub struct And<F, G> {
    f: F,
    g: G,
}

impl<I: Input, F: Parser<I>, G: Parser<I>> Parser<I> for And<F, G> {
    type Output = (F::Output, G::Output);

    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, a) = self.f.parse(i)?;
        let (i, b) = self.g.parse(i)?;

        Ok((i, (a, b)))
    }
}

#[derive(Clone)]
pub struct AndThen<P, F> {
    parser: P,
    f: F,
}
impl<I: Input, A, B, F: Fn(A) -> Q + Clone, P: Parser<I, Output = A>, Q: Parser<I, Output = B>>
    Parser<I> for AndThen<P, F>
{
    type Output = B;

    fn parse(&self, i: I) -> ParseResult<I, B> {
        let (i, a) = self.parser.parse(i)?;
        let (i, b) = (self.f)(a).parse(i)?;
        Ok((i, b))
    }
}

#[derive(Clone)]
pub struct Or<F, G> {
    f: F,
    g: G,
}
impl<I: Input, O, F: Parser<I, Output = O>, G: Parser<I, Output = O>> Parser<I> for Or<F, G> {
    type Output = F::Output;
    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        self.f.parse(i.clone()).or_else(|_| self.g.parse(i))
    }
}

pub trait Parser<I: Input>: Clone {
    type Output;

    // Required method
    fn parse(&self, input: I) -> ParseResult<I, Self::Output>;

    // Provided methods
    fn map<F: Fn(Self::Output) -> B, B>(self, f: F) -> Map<Self, F> {
        Map {
            parser: self,
            func: f,
        }
    }
    fn and<G: Parser<I, Output = O>, O>(self, g: G) -> And<Self, G> {
        And { f: self, g: g }
    }
    fn and_then<B, Q: Parser<I, Output = B>, F: Fn(Self::Output) -> Q>(
        self,
        f: F,
    ) -> AndThen<Self, F> {
        AndThen { parser: self, f: f }
    }

    fn ap<A, O, F: Fn(A) -> O, PA: Parser<I, Output = A>>(self, pa: PA) -> Ap<Self, PA>
    where
        Self: Parser<I, Output = F>,
    {
        Ap { pf: self, pa: pa }
    }

    fn or<G: Parser<I, Output = Self::Output>>(self, g: G) -> Or<Self, G> {
        Or { f: self, g: g }
    }
    fn satisfy<F: Fn(&Self::Output) -> bool>(self, f: F) -> Satisfy<Self, F> {
        Satisfy { parser: self, f: f }
    }
    fn many(self) -> Many<Self> {
        Many { parser: self }
    }
    fn some(self) -> Some<Self> {
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
    fn skip_many(self) -> SkipMany<Self> {
        SkipMany { parser: self }
    }
    fn skip_some(self) -> SkipSome<Self> {
        SkipSome { parser: self }
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
    fn sep_end_by<S>(self, sep: S) -> SepEndBy<Self, S> {
        SepEndBy { parser: self, sep }
    }
    fn sep_end_by_1<S>(self, sep: S) -> SepEndBy1<Self, S> {
        SepEndBy1 { parser: self, sep }
    }
    fn end_by<S>(self, sep: S) -> EndBy<Self, S> {
        EndBy { parser: self, sep }
    }
    fn end_by_1<S>(self, sep: S) -> EndBy1<Self, S> {
        EndBy1 { parser: self, sep }
    }
    fn count(self, n: u32) -> Count<Self> {
        Count {
            parser: self,
            count: n,
        }
    }
    fn many_till<Q>(self, end: Q) -> ManyTill<Self, Q> {
        ManyTill { parser: self, end }
    }
    fn not_followed_by<Q>(self, follow: Q) -> NotFollowedBy<Self, Q> {
        NotFollowedBy {
            parser: self,
            follow,
        }
    }
    fn void(self) -> Void<Self> {
        Void { parser: self }
    }
}

impl<I: Input, P: Parser<I>> Parser<I> for &P {
    type Output = P::Output;
    fn parse(&self, input: I) -> ParseResult<I, Self::Output> {
        (*self).parse(input)
    }
}

#[derive(Clone)]
pub struct AnyChar<I>(PhantomData<I>);
impl<I: Input<Item = char>> Parser<I> for AnyChar<I> {
    type Output = I::Item;

    fn parse(&self, input: I) -> ParseResult<I, Self::Output> {
        let mut i = I::iter(&input);
        match i.next() {
            None => Result::Err(ParseErr::EOF()),
            Some(c) => Result::Ok((I::from_iter(&i), c)),
        }
    }
}

pub fn any_char<I: Input<Item = char>>() -> AnyChar<I> {
    AnyChar(PhantomData)
}

pub fn char<I: Input<Item = char>>(c: char) -> impl Parser<I, Output = char> {
    lift(move |i| {
        let (i, c_) = any_char().parse(i)?;
        if c == c_ {
            Ok((i, c))
        } else {
            Err(ParseErr::Expected(c))
        }
    })
}

#[derive(Clone)]
pub struct StringParser<I> {
    target: String,
    phantom: PhantomData<I>,
}
impl<I: Input<Item = char>> Parser<I> for StringParser<I> {
    type Output = String;

    fn parse(&self, input: I) -> ParseResult<I, Self::Output> {
        let mut i = input;
        for c in self.target.chars() {
            match char(c).parse(i) {
                Result::Ok((ii, _)) => {
                    i = ii;
                }
                Result::Err(e) => return Result::Err(e),
            }
        }
        Ok((i, self.target.clone()))
    }
}

pub fn string<I: Input<Item = char>>(target: String) -> StringParser<I> {
    StringParser {
        target,
        phantom: PhantomData,
    }
}

#[derive(Clone)]
pub struct EOF;
impl<I: Input<Item: Debug>> Parser<I> for EOF {
    type Output = ();

    fn parse(&self, input: I) -> ParseResult<I, Self::Output> {
        let mut i = I::iter(&input);
        match i.next() {
            None => Ok((input.to_owned(), ())),
            Some(c) => Err(ParseErr::Unexpected(format!("Expected EOF, got {:?}", c))),
        }
    }
}

pub fn eof<I: Input<Item: Debug>>() -> EOF {
    EOF
}

#[derive(Clone)]
pub struct LiftA2<F, PA, PB> {
    f: F,
    pa: PA,
    pb: PB,
}
impl<
    A,
    B,
    I: Input,
    O,
    F: Fn(A, B) -> O + Clone,
    PA: Parser<I, Output = A>,
    PB: Parser<I, Output = B>,
> Parser<I> for LiftA2<F, PA, PB>
{
    type Output = O;
    fn parse(&self, i: I) -> ParseResult<I, Self::Output> {
        let (i, a) = self.pa.parse(i)?;
        let (i, b) = self.pb.parse(i)?;
        Ok((i, (self.f)(a, b)))
    }
}

pub fn lift_a_2<
    A,
    B,
    I: Input,
    O,
    F: Fn(A, B) -> O,
    PA: Parser<I, Output = A>,
    PB: Parser<I, Output = B>,
>(
    f: F,
    pa: PA,
    pb: PB,
) -> LiftA2<F, PA, PB> {
    LiftA2 { f, pa, pb }
}

pub fn id<A>(a: A) -> A {
    a
}

pub fn foo<I: Input<Item = char>>() -> impl Parser<I, Output = String> {
    let f = |t: (char, char)| format!("{:?}{:?}", t.0, t.1);
    fn a<I: Input<Item = char>>() -> impl Parser<I, Output = (char, char)> {
        char('a')
            .and(char('b'))
            .or(char('c').and(char('d')))
            .and_then(|t| {
                char('e')
                    .surround(char('\''))
                    .bracket(char('['), char(']'))
                    .map(move |_| t)
            })
    }
    fn b<I: Input<Item = char>>() -> impl Parser<I, Output = (char, char)> {
        a().and_then(|t| char('f').some().map(move |_| t))
            .and_then(|t| {
                unexpected("unexpected".to_string())
                    .or(char('g').sep_end_by_1(char('r')))
                    .map(move |_| t)
            })
            .and_then(|t| char('h').optional().map(move |_| t))
            .and_then(|t| char('f').skip_optional().map(move |_| t))
            .and_then(|t| char('x').sep_by_1(char('y')).map(move |_| t))
    }
    fn c<I: Input<Item = char>>() -> impl Parser<I, Output = (char, char)> {
        b().and_then(|t| {
            satisfy(any_char(), |c| *c == 'z')
                .skip_some()
                .and_then(|a| pure(move || a))
                .map(move |_| t)
        })
        .and_then(|t| char('x').seq(char('y')).many().map(move |_| t))
        .and_then(|t| char('z').and(char('y')).map(move |_| t))
        .and_then(|t| char('z').seq(char('x')).skip_some().map(move |_| t))
    }
    fn d<I: Input<Item = char>>() -> impl Parser<I, Output = (char, char)> {
        c().and_then(|t| {
            char('y')
                .seq(char('x'))
                .not_followed_by(eof::<I>())
                .map(move |_| t)
        })
        .and_then(|t| char('x').seq(char('y')).many().map(move |_| t))
        .and_then(|t| char('y').seq(char('z')).count(1).map(move |_| t))
        .and_then(|t| char('x').many_till(char('z')).map(move |_| t))
        .and_then(|t| char('y').end_by_1(char('z')).map(move |_| t))
    }
    fn e<I: Input<Item = char>>() -> impl Parser<I, Output = Option<char>> {
        let x = char('a')
            .map(|a| move |b| format!("{} {}", a, b))
            .ap(char('b'))
            .void()
            .optional();
        let y = lift_a_2(|_, b| id(b), char('a'), char('b')).optional();
        char(' ')
            .skip_many()
            .sep_by(char('!'))
            .sep_end_by(char('@'))
            .end_by(char('#'))
            .optional()
            .seq(x)
            .seq(y)
    }
    d().and_then(move |t| {
        let e_ = e();
        e_.clone().seq(e_).map(move |_| f(t))
    })
}
