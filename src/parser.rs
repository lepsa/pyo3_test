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
use std::{collections::LinkedList, fmt::Debug, marker::PhantomData, str::Chars};

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
impl<
    I: Input,
    O,
    E,
    A,
    F: Fn(A) -> O + Clone,
    PA: Parser<I, Output = A, Error = E>,
    PF: Parser<I, Output = F, Error = E>,
> Parser<I> for Ap<PF, PA>
{
    type Output = O;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        self.pf
            .clone()
            .and_then(move |f: F| self.pa.clone().map(f.clone()))
            .parse(i)
    }
}

pub fn ap<
    I: Input,
    A,
    O,
    F: Fn(A) -> O + Clone,
    PF: Parser<I, Output = F, Error = E>,
    PA: Parser<I, Output = A, Error = E>,
    E,
>(
    pf: PF,
    pa: PA,
) -> Ap<PF, PA> {
    pf.ap::<A, O, F, PF, PA, E>(pa)
}

#[derive(Clone)]
pub struct Map<P, F> {
    pub parser: P,
    pub func: F,
}
impl<I: Input, O, E, P: Parser<I, Error = E>, F: Fn(P::Output) -> O + Clone> Parser<I>
    for Map<P, F>
{
    type Output = O;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, o) = self.parser.parse(i)?;
        Ok((i, (self.func)(o)))
    }
}

pub fn map<I: Input, O, E, P: Parser<I, Error = E>, F: Fn(P::Output) -> O + Clone>(
    parser: P,
    f: F,
) -> Map<P, F> {
    parser.map(f)
}

#[derive(Clone)]
pub struct Some<P> {
    pub parser: P,
}
impl<I: Input, O, E, P: Parser<I, Output = O, Error = E>> Parser<I> for Some<P> {
    type Output = LinkedList<P::Output>;
    type Error = P::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, o) = self.parser.parse(i)?;
        let (i, mut l) = many(self.parser.clone()).parse(&i)?;
        l.push_front(o);
        Ok((i, l))
    }
}

pub fn some<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(p: P) -> Some<P> {
    p.some()
}

#[derive(Clone)]
pub struct Many<P> {
    pub parser: P,
}
impl<I: Input, O, E, P: Parser<I, Output = O, Error = E>> Parser<I> for Many<P> {
    type Output = LinkedList<P::Output>;
    type Error = P::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self.parser.parse(i) {
            Ok((i, o)) => {
                let (i, mut l) = many(self.parser.clone()).parse(&i)?;
                l.push_front(o);
                Ok((i, l))
            }
            Err(_) => Ok((i.to_owned(), LinkedList::new())),
        }
    }
}

pub fn many<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(p: P) -> Many<P> {
    p.many()
}

#[derive(Clone)]
pub struct Pure<I, O, E> {
    value: O,
    phantom: PhantomData<(I, E)>,
}
impl<I: Input, O: Clone, E: Clone> Parser<I> for Pure<I, O, E> {
    type Output = O;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        Ok((i.clone(), self.value.clone()))
    }
}

pub fn pure<I: Input, O: Clone, E: Clone>(o: O) -> Pure<I, O, E> {
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
impl<
    I: Input,
    P: Parser<I, Output: Debug + Clone, Error: From<String>>,
    F: Fn(P::Output) -> bool + Clone,
> Parser<I> for Satisfy<P, F>
{
    type Output = P::Output;
    type Error = P::Error;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (input, o) = self.parser.parse(i)?;
        if (self.f)(o.clone()) {
            Ok((input, o))
        } else {
            Result::Err(format!("Unexpected input: {:?}", o).into())
        }
    }
}

fn satisfy<
    I: Input,
    P: Parser<I, Output: Debug + Clone, Error: From<String>>,
    F: Fn(P::Output) -> bool + Clone,
>(
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
impl<I: Input, E, P: Parser<I, Error = E>, A: Parser<I, Error = E>> Parser<I>
    for Surrounded<P, A>
{
    type Output = P::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, _) = self.surround.parse(&i)?;
        let (i, o) = self.parser.parse(&i)?;
        let (i, _) = self.surround.parse(&i)?;

        Ok((i, o))
    }
}

pub fn surround<I: Input, O, E, P: Parser<I, Output = O, Error = E>, S: Parser<I, Error = E>>(
    parser: P,
    surround: S,
) -> Surrounded<P, S> {
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

pub fn bracket<
    I: Input,
    O,
    E,
    P: Parser<I, Output = O, Error = E>,
    B: Parser<I, Error = E>,
    K: Parser<I, Error = E>,
>(
    parser: P,
    brac: B,
    ket: K,
) -> Bracket<P, B, K> {
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

pub fn optional<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(parser: P) -> Optional<P> {
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

pub fn skip_optional<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(
    parser: P,
) -> SkipOptional<P> {
    parser.skip_optional()
}

#[derive(Clone)]
pub struct SkipMany<P> {
    parser: P,
}
impl<I: Input, E, P: Parser<I, Error = E>> Parser<I> for SkipMany<P> {
    type Output = ();
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self
            .parser
            .clone()
            .seq(skip_many(self.parser.clone()))
            .parse(i)
        {
            Err(_) => Ok((i.to_owned(), ())),
            o => o,
        }
    }
}

pub fn skip_many<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(parser: P) -> SkipMany<P> {
    parser.skip_many()
}

#[derive(Clone)]
pub struct SkipSome<P> {
    parser: P,
}
impl<I: Input, E, P: Parser<I, Error = E>> Parser<I> for SkipSome<P> {
    type Output = ();
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, _) = self.parser.clone().parse(i)?;
        skip_many(self.parser.clone()).parse(&i)
    }
}

pub fn skip_some<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(parser: P) -> SkipSome<P> {
    parser.skip_some()
}

#[derive(Clone)]
pub struct NotFollowedBy<P, Q> {
    parser: P,
    follow: Q,
}
impl<
    I: Input,
    O,
    E: From<String>,
    P: Parser<I, Output = O, Error = E>,
    Q: Parser<I, Output: Debug, Error = E>,
> Parser<I> for NotFollowedBy<P, Q>
{
    type Output = P::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, o) = self.parser.clone().parse(i)?;
        match self.follow.parse(&i) {
            Err(_) => Ok((i.to_owned(), o)),
            Ok((_, e)) => Err(format!("Unexpected: {:?}", e).into()),
        }
    }
}

pub fn not_followed_by<
    I: Input,
    O,
    QO,
    E,
    P: Parser<I, Output = O, Error = E>,
    Q: Parser<I, Output = QO, Error = E>,
>(
    parser: P,
    follow: Q,
) -> NotFollowedBy<P, Q> {
    parser.not_followed_by(follow)
}

#[derive(Clone)]
pub struct Unexpected<E> {
    message: String,
    phantom: PhantomData<E>,
}
impl<I: Input, E: From<String> + Clone> Parser<I> for Unexpected<E> {
    type Output = ();
    type Error = E;

    fn parse(&self, _i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        Err(self.message.clone().into())
    }
}

pub fn unexpected<I: Input, O, E>(message: String) -> Unexpected<O> {
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
impl<I: Input, E, P: Parser<I, Error = E>> Parser<I> for Count<P> {
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let parser = self.parser.clone();
        match self.count {
            0 => Ok((i.to_owned(), LinkedList::new())),
            n => {
                let (i, o) = self.parser.parse(i)?;
                let (i, mut l) = parser.count(n - 1).parse(&i)?;
                l.push_front(o);
                Ok((i, l))
            }
        }
    }
}

pub fn count<I: Input, O, E, P: Parser<I, Output = O, Error = E>>(parser: P, n: u32) -> Count<P> {
    parser.count(n)
}

#[derive(Clone)]
pub struct ManyTill<P, Q> {
    parser: P,
    end: Q,
}
impl<I: Input, E, P: Parser<I, Error = E>, Q: Parser<I, Error = E>> Parser<I> for ManyTill<P, Q> {
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        match self.end.parse(i) {
            Ok((i, _)) => Ok((i, LinkedList::new())),
            Err(_) => {
                let (i, o) = self.parser.parse(i)?;
                let (i, mut l) = many_till(self.parser.clone(), self.end.clone()).parse(&i)?;
                l.push_front(o);
                Ok((i, l))
            }
        }
    }
}

pub fn many_till<I: Input, O, E, P: Parser<I, Output = O, Error = E>, Q: Parser<I, Error = E>>(
    parser: P,
    end: Q,
) -> ManyTill<P, Q> {
    parser.many_till(end)
}

#[derive(Clone)]
pub struct SepBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, E: Clone, O: Clone, P: Parser<I, Output = O, Error = E>, S: Parser<I, Error = E>>
    Parser<I> for SepBy<P, S>
{
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        self.parser.clone()
            .sep_by_1(self.sep.clone())
            .or(pure(LinkedList::new()))
            .parse(i)
    }
}

pub fn sep_by<
    I: Input,
    O: Clone,
    E: Clone,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
>(
    parser: P,
    sep: S,
) -> SepBy<P, S> {
    parser.sep_by(sep)
}

fn prepend_list<O: Clone>(o: O, mut l: LinkedList<O>) -> LinkedList<O> {
    l.push_front(o);
    l.clone()
}

#[derive(Clone)]
pub struct EndBy1<P, S> {
    parser: P,
    sep: S,
}

impl<
    I: Input,
    O: Clone,
    SO,
    E: Clone,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Output = SO, Error = E>,
> Parser<I> for EndBy1<P, S>
{
    type Output = LinkedList<O>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let o = or(
            seq(
                self.sep.clone(),
                sep_end_by(self.parser.clone(), self.sep.clone()),
            ),
            pure(LinkedList::new()),
        );
        let p = lift_a_2(prepend_list, self.parser.clone(), o);
        p.parse(i)
    }
}

pub fn end_by_1<
    I: Input,
    O: Clone,
    E: Clone,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
>(
    parser: P,
    sep: S,
) -> EndBy1<P, S> {
    parser.end_by_1(sep)
}

#[derive(Clone)]
pub struct EndBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O: Clone, E: Clone, P: Parser<I, Output = O, Error = E>, S: Parser<I, Error = E>>
    Parser<I> for EndBy<P, S>
{
    type Output = LinkedList<O>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        self.parser.clone()
            .sep_end_by_1(self.sep.clone())
            .or(pure(LinkedList::new()))
            .parse(i)
    }
}

pub fn end_by<
    I: Input,
    O: Clone,
    E: Clone,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
>(
    parser: P,
    sep: S,
) -> EndBy<P, S> {
    parser.end_by(sep)
}

#[derive(Clone)]
pub struct SepEndBy<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O: Clone, E: Clone, P: Parser<I, Output = O, Error = E>, S: Parser<I, Error = E>>
    Parser<I> for SepEndBy<P, S>
{
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        self.parser.clone()
            .sep_end_by_1(self.sep.clone())
            .or(pure(LinkedList::new()))
            .parse(i)
    }
}

pub fn sep_end_by<
    I: Input,
    O: Clone,
    E: Clone,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
>(
    parser: P,
    sep: S,
) -> SepEndBy<P, S> {
    parser.sep_end_by(sep)
}

#[derive(Clone)]
pub struct SepEndBy1<P, S> {
    parser: P,
    sep: S,
}

impl<I: Input, O: Clone, E: Clone, P: Parser<I, Output = O, Error = E>, S: Parser<I, Error = E>>
    Parser<I> for SepEndBy1<P, S>
{
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        some(
            self.parser.clone()
                .and_then(move |o: O| self.sep.clone().map(move |_| o.clone())),
        )
        .or(pure(LinkedList::new()))
        .parse(i)
    }
}

pub fn sep_end_by_1<
    I: Input,
    O: Clone,
    E: Clone,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
>(
    parser: P,
    sep: S,
) -> SepEndBy1<P, S> {
    parser.sep_end_by_1(sep)
}

#[derive(Clone)]
pub struct SepBy1<P, S> {
    parser: P,
    sep: S,
}

impl<
    I: Input,
    O: Clone,
    SO,
    E,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Output = SO, Error = E>,
> Parser<I> for SepBy1<P, S>
{
    type Output = LinkedList<P::Output>;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let pa = many(self.sep.clone().seq(self.parser.clone()));
        lift_a_2(prepend_list, self.parser.clone(), pa).parse(i)
    }
}

pub fn sep_by_1<
    I: Input,
    O: Clone,
    E,
    P: Parser<I, Output = O, Error = E>,
    S: Parser<I, Error = E>,
>(
    parser: P,
    sep: S,
) -> SepBy1<P, S> {
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

pub fn seq<I: Input, O, E, P: Parser<I, Error = E>, Q: Parser<I, Output = O, Error = E>>(
    a: P,
    b: Q,
) -> Seq<P, Q> {
    a.seq(b)
}

#[derive(Clone)]
pub struct And<F, G> {
    f: F,
    g: G,
}

pub fn and<
    I: Input,
    A,
    B,
    E,
    F: Parser<I, Output = A, Error = E>,
    G: Parser<I, Output = B, Error = E>,
>(
    f: F,
    g: G,
) -> And<F, G> {
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
    F: Fn(A) -> Q + Clone,
> Parser<I> for AndThen<P, F>
{
    type Output = Q::Output;
    type Error = E;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let (i, a) = self.parser.parse(i)?;
        let (i, b) = (self.f)(a).parse(&i)?;
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
    F: Fn(A) -> Q + Clone,
>(
    parser: P,
    func: F,
) -> AndThen<P, F> {
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
        self.f.parse(i).or_else(|_| self.g.parse(i))
    }
}

pub fn or<
    I: Input,
    O,
    E,
    F: Parser<I, Output = O, Error = E>,
    G: Parser<I, Output = O, Error = E>,
>(
    f: F,
    g: G,
) -> Or<F, G> {
    f.or(g)
}

pub trait Parser<I: Input>: Clone + Sized {
    type Output;
    type Error;

    // Required method
    fn parse(&self, input: &I) -> ParseResult<I, Self::Output, Self::Error>;

    // Provided methods
    fn map<F: Fn(Self::Output) -> O2, O2>(self, f: F) -> Map<Self, F> {
        Map {
            parser: self,
            func: f,
        }
    }
    fn and<G: Parser<I, Output = O, Error = Self::Error>, O>(self, g: G) -> And<Self, G> {
        And { f: self, g: g }
    }
    fn and_then<F>(self, f: F) -> AndThen<Self, F> {
        AndThen { parser: self, f: f }
    }

    fn ap<A, O, F: Fn(A) -> O, PF: Parser<I, Output = F>, PA: Parser<I, Output = A, Error = E>, E>(
        self,
        pa: PA,
    ) -> Ap<Self, PA>
    where
        Self: Parser<I, Output: Fn(A) -> O, Error = E>,
    {
        Ap { pf: self, pa: pa }
    }

    fn or<G: Parser<I, Output = Self::Output, Error = Self::Error>>(self, g: G) -> Or<Self, G> {
        Or { f: self, g: g }
    }
    fn satisfy<F: Fn(Self::Output) -> bool + Clone>(self, f: F) -> Satisfy<Self, F> {
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
    fn void(self) -> impl Parser<I, Output = (), Error = Self::Error> {
        self.map(|_| ())
    }
}

#[derive(Clone)]
pub struct AnyChar<I, E>(PhantomData<(I, E)>);
impl<I: Input<Item = char>, E: From<String> + Clone> Parser<I> for AnyChar<I, E> {
    type Output = I::Item;
    type Error = E;

    fn parse(&self, input: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let mut i = I::iter(input);
        match i.next() {
            None => Result::Err(format!("End Of File").into()),
            Some(c) => Result::Ok((I::from_iter(&i), c)),
        }
    }
}

pub fn any_char<I: Input<Item = char>, E:From<String> + Clone>() -> AnyChar<I, E> {
    AnyChar(PhantomData)
}

pub fn char<I: Input<Item = char>, E: From<String> + Clone>(c: char) -> impl Parser<I, Output = char, Error = E> {
    satisfy(any_char(), move |c_| c_ == c)
}

#[derive(Clone)]
pub struct EOF<E>(PhantomData<E>);
impl<I: Input<Item: Debug>, E:From<String> + Clone> Parser<I> for EOF<E> {
    type Output = ();
    type Error = E;

    fn parse(&self, input: &I) -> ParseResult<I, Self::Output, Self::Error> {
        let mut i = I::iter(input);
        match i.next() {
            None => Ok((input.to_owned(), ())),
            Some(c) => Err(format!("Expected EOF, got {:?}", c).into()),
        }
    }
}

pub fn eof<I: Input<Item: Debug>, E: From<String> + Clone>() -> EOF<E> {
    EOF(PhantomData)
}

fn lift_a_2<
    A: Clone,
    B,
    I: Input,
    O,
    F: Fn(A, B) -> O + Clone,
    PA: Parser<I, Output = A, Error = E>,
    PB: Parser<I, Output = B, Error = E>,
    E,
>(
    f: F,
    pa: PA,
    pb: PB,
) -> impl Parser<I, Output = O, Error = E> {
    pa.and_then(move |a: A| {
        let f = f.clone();
        pb.clone().map(move |b: B| f.clone()(a.clone(), b))
    })
}

pub fn foo<I: Input<Item = char>>() -> impl Parser<I, Output = String, Error = String> {
    let f = |s: String| Parser::map(count(char('f'), 3), move |_| s.clone())
    .and_then(|s: String| skip_some(char('g')).map(move |_| s.clone()))
    .and_then(|s: String| skip_optional(char('r')).map(move |_| s.clone()))
    .and_then(|s: String| {
        many_till(not_followed_by(skip_many(char('h')), char('q')), char('f'))
            .map(move |_| s.clone())
    })
    .and_then(|s: String| sep_by(char('x'), char('y')).map(move |_| s.clone()))
    .and_then(|s: String| end_by(char('z'), char('x')).map(move |_| s.clone()))
    .and_then(|s: String| end_by_1(char('y'), char('z')).map(move |_| s.clone()))
    .and_then(|s: String| sep_end_by_1(char('x'), char('y')).map(move |_| s.clone()))
    .and_then(|s: String| sep_by_1(char('z'), char('x')).map(move |_| s.clone()))
    .and_then(|s: String| {
        sep_by(char('y'), char('z').void())
            .seq(eof::<I, String>())
            .map(move |_| s.clone())
    });
    and_then(
        and(
            map(
                or(char('a').and(char('b')), char('c').and(char('d'))),
                |t: (char, char)| format!("({}. {})", t.0, t.1),
            ),
            ap(
                pure(|l: LinkedList<Option<char>>| format!("{:?}", l)),
                some(bracket(
                    optional(surround(char('e'), char('\''))),
                    char('['),
                    not_followed_by(
                        char(']'),
                        unexpected::<I, String, String>("unexpected fail".to_string()),
                    ),
                )),
            ),
        ),
        |t: (String, String)| pure(format!("{:?} and {:?}", t.0, t.1)),
    ).and_then(f)
}
