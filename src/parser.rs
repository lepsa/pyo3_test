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
use std::{
    cell::RefCell, collections::LinkedList, fmt::Debug, marker::PhantomData, rc::Rc, str::Chars,
};

#[derive(Clone, Debug)]
pub enum ParseErr {
    EOF,
    Unexpected(String),
    Expected(char),
}

impl ParseErr {
    pub fn to_string(&self) -> String {
        match self {
            ParseErr::EOF => "End Of File".to_string(),
            ParseErr::Unexpected(msg) => format!("Unexpected: {}", msg),
            ParseErr::Expected(c) => format!("Expected to see char: {}", c),
        }
    }
}

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

pub type ParseResult<I, O> = Result<(I, O), ParseErr>;

pub struct Ap<'f, 'a, PF, PA> {
    pub pf: &'f PF,
    pub pa: &'a PA,
}
impl<'f, 'a, I: Input, O, A, F: Fn(&A) -> O, PA: Parser<I, Output = A>, PF: Parser<I, Output = F>>
    Parser<I> for Ap<'f, 'a, PF, PA>
{
    type Output = O;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, f) = self.pf.parse(i)?;
        let (i, a) = self.pa.parse(&i)?;
        Ok((i, f(&a)))
    }
}

pub fn ap<
    'f,
    'a,
    I: Input,
    A,
    O,
    F: Fn(&A) -> O,
    PF: Parser<I, Output = F>,
    PA: Parser<I, Output = A>,
>(
    pf: &'f PF,
    pa: &'a PA,
) -> Ap<'f, 'a, PF, PA> {
    pf.ap::<'f, 'a, A, O, F, PF, PA>(pa)
}

pub struct Map<'p, 'f, P, F> {
    pub parser: &'p P,
    pub func: &'f F,
}
impl<'p, 'f, I: Input, O, P: Parser<I>, F: Fn(&P::Output) -> O> Parser<I> for Map<'p, 'f, P, F> {
    type Output = O;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, o) = self.parser.parse(i)?;
        Ok((i, (self.func)(&o)))
    }
}

pub fn map<'p, 'f, I: Input, O, P: Parser<I>, F: Fn(&P::Output) -> O>(
    parser: &'p P,
    f: &'f F,
) -> Map<'p, 'f, P, F> {
    parser.map(f)
}

pub struct Some<'p, P> {
    pub parser: &'p P,
}
impl<'p, I: Input, O, P: Parser<I, Output = O>> Parser<I> for Some<'p, P> {
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, o) = self.parser.parse(i)?;
        let (i, l) = many(self.parser).parse(&i)?;
        l.borrow_mut().push_front(o);
        Ok((i, l))
    }
}

pub fn some<'p, I: Input, O, P: Parser<I, Output = O>>(p: &'p P) -> Some<'p, P> {
    p.some()
}

pub struct Many<'p, P> {
    pub parser: &'p P,
}
impl<'p, I: Input, O, P: Parser<I, Output = O>> Parser<I> for Many<'p, P> {
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        match self.parser.parse(i) {
            Ok((i, o)) => {
                let (i, l) = many(self.parser).parse(&i)?;
                l.borrow_mut().push_front(o);
                Ok((i, l))
            }
            Err(_) => Ok((i.to_owned(), Rc::new(RefCell::new(LinkedList::new())))),
        }
    }
}

pub fn many<'p, I: Input, O, P: Parser<I, Output = O>>(p: &'p P) -> Many<'p, P> {
    p.many()
}

pub struct Lift<F> {
    func: F,
}
impl<I: Input, O, F: Fn(&I) -> ParseResult<I, O>> Parser<I> for Lift<F> {
    type Output = O;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        (self.func)(i)
    }
}

pub fn lift<I: Input, O, F: Fn(&I) -> ParseResult<I, O>>(func: F) -> Lift<F> {
    Lift { func }
}

pub fn pure<I: Input, O>(o: O) -> impl Parser<I, Output = Rc<RefCell<O>>> {
    let rc_o = Rc::new(RefCell::new(o));
    lift(move |i: &I| Ok((i.clone(), Rc::clone(&rc_o))))
}

pub struct Satisfy<'p, 'f, P, F> {
    parser: &'p P,
    f: &'f F,
}
impl<'p, 'f, I: Input, P: Parser<I, Output: Debug>, F: Fn(&P::Output) -> bool> Parser<I>
    for Satisfy<'p, 'f, P, F>
{
    type Output = P::Output;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (input, o) = self.parser.parse(i)?;
        if (self.f)(&o) {
            Ok((input, o))
        } else {
            Result::Err(ParseErr::Unexpected(format!("Unexpected input: {:?}", o)))
        }
    }
}

fn satisfy<'p, 'f, I: Input, P: Parser<I, Output: Debug>, F: Fn(&P::Output) -> bool>(
    parser: &'p P,
    f: &'f F,
) -> Satisfy<'p, 'f, P, F> {
    parser.satisfy(f)
}

pub struct Surrounded<'p, 'a, P, A> {
    parser: &'p P,
    surround: &'a A,
}
impl<'p, 'a, I: Input, P: Parser<I>, A: Parser<I>> Parser<I> for Surrounded<'p, 'a, P, A> {
    type Output = P::Output;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.surround.parse(&i)?;
        let (i, o) = self.parser.parse(&i)?;
        let (i, _) = self.surround.parse(&i)?;

        Ok((i, o))
    }
}

pub fn surround<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    surround: &'s S,
) -> Surrounded<'p, 's, P, S> {
    parser.surround(surround)
}

pub struct Bracket<'p, 'b, 'k, P, B, K> {
    parser: &'p P,
    brac: &'b B,
    ket: &'k K,
}
impl<'p, 'b, 'k, I: Input, P: Parser<I>, B: Parser<I>, K: Parser<I>> Parser<I>
    for Bracket<'p, 'b, 'k, P, B, K>
{
    type Output = P::Output;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.brac.parse(&i)?;
        let (i, o) = self.parser.parse(&i)?;
        let (i, _) = self.ket.parse(&i)?;

        Ok((i, o))
    }
}

pub fn bracket<'p, 'b, 'k, I: Input, O, P: Parser<I, Output = O>, B: Parser<I>, K: Parser<I>>(
    parser: &'p P,
    brac: &'b B,
    ket: &'k K,
) -> Bracket<'p, 'b, 'k, P, B, K> {
    parser.bracket(brac, ket)
}

pub struct Optional<'p, P> {
    parser: &'p P,
}
impl<'p, I: Input, P: Parser<I>> Parser<I> for Optional<'p, P> {
    type Output = Option<P::Output>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        match self.parser.parse(&i) {
            Err(_) => Ok((i.to_owned(), None)),
            Ok((i, o)) => Ok((i, Some(o))),
        }
    }
}

pub fn optional<'p, I: Input, O, P: Parser<I, Output = O>>(parser: &'p P) -> Optional<'p, P> {
    parser.optional()
}

pub struct SkipOptional<'p, P> {
    parser: &'p P,
}
impl<'p, I: Input, P: Parser<I>> Parser<I> for SkipOptional<'p, P> {
    type Output = ();

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        match self.parser.parse(&i) {
            Err(_) => Ok((i.to_owned(), ())),
            Ok((i, _)) => Ok((i, ())),
        }
    }
}

pub fn skip_optional<'p, I: Input, O, P: Parser<I, Output = O>>(
    parser: &'p P,
) -> SkipOptional<'p, P> {
    parser.skip_optional()
}

pub struct SkipMany<'p, P> {
    parser: &'p P,
}
impl<'p, I: Input, P: Parser<I>> Parser<I> for SkipMany<'p, P> {
    type Output = ();

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        match self.parser.seq(&skip_many(self.parser)).parse(i) {
            Err(_) => Ok((i.to_owned(), ())),
            o => o,
        }
    }
}

pub fn skip_many<'p, I: Input, O, P: Parser<I, Output = O>>(parser: &'p P) -> SkipMany<'p, P> {
    parser.skip_many()
}

pub struct SkipSome<'p, P> {
    parser: &'p P,
}
impl<'p, I: Input, P: Parser<I>> Parser<I> for SkipSome<'p, P> {
    type Output = ();

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.parser.parse(i)?;
        skip_many(self.parser).parse(&i)
    }
}

pub fn skip_some<'p, I: Input, O, P: Parser<I, Output = O>>(parser: &'p P) -> SkipSome<'p, P> {
    parser.skip_some()
}

pub struct NotFollowedBy<'p, 'q, P, Q> {
    parser: &'p P,
    follow: &'q Q,
}
impl<'p, 'q, I: Input, O, P: Parser<I, Output = O>, Q: Parser<I, Output: Debug>> Parser<I>
    for NotFollowedBy<'p, 'q, P, Q>
{
    type Output = P::Output;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, o) = self.parser.parse(i)?;
        match self.follow.parse(&i) {
            Err(_) => Ok((i.to_owned(), o)),
            Ok((_, e)) => Err(ParseErr::Unexpected(format!("Unexpected: {:?}", e))),
        }
    }
}

pub fn not_followed_by<
    'p,
    'q,
    I: Input,
    O,
    QO,
    P: Parser<I, Output = O>,
    Q: Parser<I, Output = QO>,
>(
    parser: &'p P,
    follow: &'q Q,
) -> NotFollowedBy<'p, 'q, P, Q> {
    parser.not_followed_by(follow)
}

pub struct Unexpected {
    message: String,
}
impl<I: Input> Parser<I> for Unexpected {
    type Output = ();

    fn parse(&self, _i: &I) -> ParseResult<I, Self::Output> {
        Err(ParseErr::Unexpected(self.message.clone()))
    }
}

pub fn unexpected<I: Input>(message: String) -> Unexpected {
    Unexpected { message }
}

pub struct Count<'p, P> {
    parser: &'p P,
    count: u32,
}
impl<'p, I: Input, O, P: Parser<I, Output = O>> Parser<I> for Count<'p, P> {
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        match self.count {
            0 => Ok((i.to_owned(), Rc::new(RefCell::new(LinkedList::new())))),
            n => {
                let (i, o) = self.parser.parse(i)?;
                let (i, l) = self.parser.count(n - 1).parse(&i)?;
                l.borrow_mut().push_front(o);
                Ok((i, l))
            }
        }
    }
}

pub fn count<'p, I: Input, O, P: Parser<I, Output = O>>(parser: &'p P, n: u32) -> Count<'p, P> {
    parser.count(n)
}

pub struct ManyTill<'p, 'q, P, Q> {
    parser: &'p P,
    end: &'q Q,
}
impl<'p, 'q, I: Input, O, P: Parser<I, Output = O>, Q: Parser<I>> Parser<I>
    for ManyTill<'p, 'q, P, Q>
{
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        match self.end.parse(i) {
            Ok((i, _)) => Ok((i, Rc::new(RefCell::new(LinkedList::new())))),
            Err(_) => {
                let (i, o) = self.parser.parse(i)?;
                let (i, l) = many_till(self.parser, self.end).parse(&i)?;
                l.borrow_mut().push_front(o);
                Ok((i, l))
            }
        }
    }
}

pub fn many_till<'p, 'q, I: Input, O, P: Parser<I, Output = O>, Q: Parser<I>>(
    parser: &'p P,
    end: &'q Q,
) -> ManyTill<'p, 'q, P, Q> {
    parser.many_till(end)
}

pub struct SepBy<'p, 's, P, S> {
    parser: &'p P,
    sep: &'s S,
}

impl<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>> Parser<I>
    for SepBy<'p, 's, P, S>
{
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        self.parser
            .sep_by_1(self.sep)
            .or(&pure(LinkedList::new()))
            .parse(i)
    }
}

pub fn sep_by<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    sep: &'s S,
) -> SepBy<'p, 's, P, S> {
    parser.sep_by(sep)
}

pub struct EndBy1<'p, 's, P, S> {
    parser: &'p P,
    sep: &'s S,
}

impl<'p, 's, I: Input, O, SO, P: Parser<I, Output = O>, S: Parser<I, Output = SO>> Parser<I>
    for EndBy1<'p, 's, P, S>
{
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, a) = self.parser.parse(i)?;
        let (i, l) = or(
            &self.sep.seq(&sep_end_by(self.parser, self.sep)),
            &pure(LinkedList::new()),
        )
        .parse(&i)?;

        l.borrow_mut().push_front(a);
        Ok((i, l))
    }
}

pub fn end_by_1<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    sep: &'s S,
) -> EndBy1<'p, 's, P, S> {
    parser.end_by_1(sep)
}

pub struct EndBy<'p, 's, P, S> {
    parser: &'p P,
    sep: &'s S,
}

impl<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>> Parser<I>
    for EndBy<'p, 's, P, S>
{
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        self.parser
            .sep_end_by_1(self.sep)
            .or(&pure(LinkedList::new()))
            .parse(i)
    }
}

pub fn end_by<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    sep: &'s S,
) -> EndBy<'p, 's, P, S> {
    parser.end_by(sep)
}

pub struct SepEndBy<'p, 's, P, S> {
    parser: &'p P,
    sep: &'s S,
}

impl<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>> Parser<I>
    for SepEndBy<'p, 's, P, S>
{
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        self.parser
            .sep_end_by_1(self.sep)
            .or(&pure(LinkedList::new()))
            .parse(i)
    }
}

pub fn sep_end_by<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    sep: &'s S,
) -> SepEndBy<'p, 's, P, S> {
    parser.sep_end_by(sep)
}

pub struct SepEndBy1<'p, 's, P, S> {
    parser: &'p P,
    sep: &'s S,
}

impl<'p, 's, I: Input, A: Clone, B, P: Parser<I, Output = A>, S: Parser<I, Output = B>> Parser<I>
    for SepEndBy1<'p, 's, P, S>
{
    type Output = Rc<RefCell<LinkedList<A>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let l = pure::<I, LinkedList<A>>(LinkedList::new());
        let sep_ = |o: &A| {
            lift(|i| {
                let (i, _) = self.sep.parse(i)?;
                Ok((i, o.to_owned()))
            })
        };
        let p = and_then(self.parser, &sep_);
        or(&some(&p), &l).parse(i)
    }
}

pub fn sep_end_by_1<'p, 's, I: Input, O, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    sep: &'s S,
) -> SepEndBy1<'p, 's, P, S> {
    parser.sep_end_by_1(sep)
}

pub struct SepBy1<'p, 's, P, S> {
    parser: &'p P,
    sep: &'s S,
}

impl<'p, 's, I: Input, O, SO, P: Parser<I, Output = O>, S: Parser<I, Output = SO>> Parser<I>
    for SepBy1<'p, 's, P, S>
{
    type Output = Rc<RefCell<LinkedList<O>>>;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, a) = self.parser.parse(i)?;
        let (i, l) = self.sep.seq(self.parser).many().parse(&i)?;
        l.borrow_mut().push_front(a);
        Ok((i, l))
    }
}

pub fn sep_by_1<'p, 's, I: Input, O: Clone, P: Parser<I, Output = O>, S: Parser<I>>(
    parser: &'p P,
    sep: &'s S,
) -> SepBy1<'p, 's, P, S> {
    parser.sep_by_1(sep)
}

pub struct Seq<'p, 'q, P, Q> {
    a: &'p P,
    b: &'q Q,
}

impl<'p, 'q, I: Input, P: Parser<I>, Q: Parser<I>> Parser<I> for Seq<'p, 'q, P, Q> {
    type Output = Q::Output;

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, _) = self.a.parse(i)?;
        self.b.parse(&i)
    }
}

pub fn seq<'p, 'q, I: Input, O, P: Parser<I>, Q: Parser<I, Output = O>>(
    a: &'p P,
    b: &'q Q,
) -> Seq<'p, 'q, P, Q> {
    a.seq(b)
}

pub struct And<'f, 'g, F, G> {
    f: &'f F,
    g: &'g G,
}

pub fn and<'f, 'g, I: Input, A, B, F: Parser<I, Output = A>, G: Parser<I, Output = B>>(
    f: &'f F,
    g: &'g G,
) -> And<'f, 'g, F, G> {
    f.and(g)
}

impl<'f, 'g, I: Input, F: Parser<I>, G: Parser<I>> Parser<I> for And<'f, 'g, F, G> {
    type Output = (F::Output, G::Output);

    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        let (i, a) = self.f.parse(i)?;
        let (i, b) = self.g.parse(&i)?;

        Ok((i, (a, b)))
    }
}

pub struct AndThen<'p, 'f, P, F> {
    parser: &'p P,
    f: &'f F,
}
impl<'p, 'f, 'a, I: Input, A, B, F: Fn(&A) -> Q, P: Parser<I, Output = A>, Q: Parser<I, Output = B>>
    Parser<I> for AndThen<'p, 'f, P, F>
{
    type Output = B;

    fn parse(&self, i: &I) -> ParseResult<I, B> {
        let (i, a) = self.parser.parse(i)?;
        let (i, b) = (self.f)(&a).parse(&i)?;
        Ok((i, b))
    }
}

pub fn and_then<
    'p,
    'f,
    I: Input,
    A,
    B,
    P: Parser<I, Output = A>,
    Q: Parser<I, Output = B>,
    F: Fn(&A) -> Q,
>(
    parser: &'p P,
    func: &'f F,
) -> AndThen<'p, 'f, P, F> {
    parser.and_then(func)
}

pub struct Or<'f, 'g, F, G> {
    f: &'f F,
    g: &'g G,
}
impl<'f, 'g, I: Input, O, F: Parser<I, Output = O>, G: Parser<I, Output = O>> Parser<I>
    for Or<'f, 'g, F, G>
{
    type Output = F::Output;
    fn parse(&self, i: &I) -> ParseResult<I, Self::Output> {
        self.f.parse(i).or_else(|_| self.g.parse(i))
    }
}

pub fn or<'f, 'g, I: Input, O, F: Parser<I, Output = O>, G: Parser<I, Output = O>>(
    f: &'f F,
    g: &'g G,
) -> Or<'f, 'g, F, G> {
    f.or(g)
}

pub trait Parser<I: Input>: Sized {
    type Output;

    // Required method
    fn parse(&self, input: &I) -> ParseResult<I, Self::Output>;

    // Provided methods
    fn map<'p, 'f, F: Fn(&Self::Output) -> B, B>(&'p self, f: &'f F) -> Map<'p, 'f, Self, F> {
        Map {
            parser: self,
            func: f,
        }
    }
    fn and<'f, 'g, G: Parser<I, Output = O>, O>(&'f self, g: &'g G) -> And<'f, 'g, Self, G> {
        And { f: self, g: g }
    }
    fn and_then<'p, 'f, B, Q: Parser<I, Output = B>, F: Fn(&Self::Output) -> Q>(
        &'p self,
        f: &'f F,
    ) -> AndThen<'p, 'f, Self, F> {
        AndThen { parser: self, f: f }
    }

    fn ap<'f, 'a, A, O, F: Fn(&A) -> O, PF: Parser<I, Output = F>, PA: Parser<I, Output = A>>(
        &'f self,
        pa: &'a PA,
    ) -> Ap<'f, 'a, Self, PA>
    where
        Self: Parser<I, Output: Fn(&A) -> O>,
    {
        Ap { pf: self, pa: pa }
    }

    fn or<'f, 'g, G: Parser<I, Output = Self::Output>>(&'f self, g: &'g G) -> Or<'f, 'g, Self, G> {
        Or { f: self, g: g }
    }
    fn satisfy<'p, 'f, F: Fn(&Self::Output) -> bool>(
        &'p self,
        f: &'f F,
    ) -> Satisfy<'p, 'f, Self, F> {
        Satisfy { parser: self, f: f }
    }
    fn many<'p>(&'p self) -> Many<'p, Self> {
        Many { parser: self }
    }
    fn some<'p>(&'p self) -> Some<'p, Self> {
        Some { parser: self }
    }
    fn surround<'p, 's, S>(&'p self, surround: &'s S) -> Surrounded<'p, 's, Self, S> {
        Surrounded {
            parser: self,
            surround,
        }
    }
    fn bracket<'p, 'b, 'k, B, K>(
        &'p self,
        brac: &'b B,
        ket: &'k K,
    ) -> Bracket<'p, 'b, 'k, Self, B, K> {
        Bracket {
            parser: self,
            brac,
            ket,
        }
    }
    fn optional<'p>(&'p self) -> Optional<'p, Self> {
        Optional { parser: self }
    }
    fn skip_optional<'p>(&'p self) -> SkipOptional<'p, Self> {
        SkipOptional { parser: self }
    }
    fn skip_many<'p>(&'p self) -> SkipMany<'p, Self> {
        SkipMany { parser: self }
    }
    fn skip_some<'p>(&'p self) -> SkipSome<'p, Self> {
        SkipSome { parser: self }
    }
    fn seq<'p, 'q, Q>(&'p self, b: &'q Q) -> Seq<'p, 'q, Self, Q> {
        Seq { a: self, b }
    }
    fn sep_by<'p, 's, S>(&'p self, sep: &'s S) -> SepBy<'p, 's, Self, S> {
        SepBy { parser: self, sep }
    }
    fn sep_by_1<'p, 's, S>(&'p self, sep: &'s S) -> SepBy1<'p, 's, Self, S> {
        SepBy1 { parser: self, sep }
    }
    fn sep_end_by<'p, 's, S>(&'p self, sep: &'s S) -> SepEndBy<'p, 's, Self, S> {
        SepEndBy { parser: self, sep }
    }
    fn sep_end_by_1<'p, 's, S>(&'p self, sep: &'s S) -> SepEndBy1<'p, 's, Self, S> {
        SepEndBy1 { parser: self, sep }
    }
    fn end_by<'p, 's, S>(&'p self, sep: &'s S) -> EndBy<'p, 's, Self, S> {
        EndBy { parser: self, sep }
    }
    fn end_by_1<'p, 's, S>(&'p self, sep: &'s S) -> EndBy1<'p, 's, Self, S> {
        EndBy1 { parser: self, sep }
    }
    fn count<'p>(&'p self, n: u32) -> Count<'p, Self> {
        Count {
            parser: self,
            count: n,
        }
    }
    fn many_till<'p, 'q, Q>(&'p self, end: &'q Q) -> ManyTill<'p, 'q, Self, Q> {
        ManyTill { parser: self, end }
    }
    fn not_followed_by<'p, 'q, Q>(&'p self, follow: &'q Q) -> NotFollowedBy<'p, 'q, Self, Q> {
        NotFollowedBy {
            parser: self,
            follow,
        }
    }
    fn void(&self) -> impl Parser<I, Output = ()> {
        lift(|i| {
            let (i, _) = self.parse(i)?;
            Ok((i, ()))
        })
    }
}

impl<I: Input, P: Parser<I>> Parser<I> for &P {
    type Output = P::Output;
    fn parse(&self, input: &I) -> ParseResult<I, Self::Output> {
        (*self).parse(input)
    }
}

pub struct AnyChar<I>(PhantomData<I>);
impl<I: Input<Item = char>> Parser<I> for AnyChar<I> {
    type Output = I::Item;

    fn parse(&self, input: &I) -> ParseResult<I, Self::Output> {
        let mut i = I::iter(input);
        match i.next() {
            None => Result::Err(ParseErr::EOF),
            Some(c) => Result::Ok((I::from_iter(&i), c)),
        }
    }
}

pub fn any_char<I: Input<Item = char>>() -> AnyChar<I> {
    AnyChar(PhantomData)
}

pub fn char<I: Input<Item = char> + 'static>(c: char) -> impl Parser<I, Output = char> {
    lift(move |i| {
        let (i, c_) = any_char().parse(i)?;
        if c == c_ {
            Ok((i, c))
        } else {
            Err(ParseErr::Expected(c))
        }
    })
}

pub struct EOF;
impl<I: Input<Item: Debug>> Parser<I> for EOF {
    type Output = ();

    fn parse(&self, input: &I) -> ParseResult<I, Self::Output> {
        let mut i = I::iter(input);
        match i.next() {
            None => Ok((input.to_owned(), ())),
            Some(c) => Err(ParseErr::Unexpected(format!("Expected EOF, got {:?}", c))),
        }
    }
}

pub fn eof<I: Input<Item: Debug>>() -> EOF {
    EOF
}

fn lift_a_2<
    A,
    B,
    I: Input,
    O,
    F: Fn(&A, &B) -> O,
    PA: Parser<I, Output = A>,
    PB: Parser<I, Output = B>,
>(
    f: F,
    pa: PA,
    pb: PB,
) -> impl Parser<I, Output = O> {
    lift(move |i| {
        let (i, a) = pa.parse(i)?;
        let (i, b) = pb.parse(&i)?;
        Ok((i, f(&a, &b)))
    })
}
/*
pub fn foo<I: Input<Item = char>>() -> impl Parser<I, Output = String> {
    let f = |s: String| {
        Parser::map(count(char('f'), 3), move |_| s.clone())
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
                    .seq(eof::<I>())
                    .map(move |_| s.clone())
            })
    };
    and_then(
        and(
            map(
                or(char('a').and(char('b')), char('c').and(char('d'))),
                |t: (char, char)| format!("({}. {})", t.0, t.1),
            ),
            &map(
                pure(|l: LinkedList<Option<char>>| format!("{:?}", l)),
                some(&bracket(
                    optional(surround(char('e'), char('\''))),
                    char('['),
                    not_followed_by(char(']'), unexpected::<I>("unexpected fail".to_string())),
                )),
            ),
        ),
        |t: (String, String)| pure(format!("{:?} and {:?}", t.0, t.1)),
    )
    .and_then(f)
}
*/
