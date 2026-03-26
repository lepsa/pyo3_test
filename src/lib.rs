mod foo;
mod parser;
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod pyo3_test {
    use std::collections::HashMap;

    use crate::{
        foo::{self, Bar},
        parser::{ParseResult, *},
    };
    use pyo3::{prelude::*, types::*};

    /// Formats the sum of two numbers as string.
    /// Mixing python macros and pure rust library code
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok(foo::sum_as_string(a, b))
    }

    // Direct python facing code
    #[pyfunction]
    fn fold_list<'py>(list: Bound<'py, PyList>) -> PyResult<i32> {
        list.iter().fold(Ok(0), |count, item| {
            count.and_then(|c| item.extract::<i32>().map(|i| c + i))
        })
    }

    // Create a basic python class called Foo
    // Using Foo as a new type wrapper around Bar to show that
    // we can mix generic Rust code and python FFI code.
    #[pyclass]
    struct Foo(Bar<Py<PyAny>>);
    // Adding functions into the class definition
    #[pymethods]
    impl Foo {
        // A writable class attribute. We have to read it as a function
        #[classattr]
        fn cls_attr() -> String {
            "cls_attr".to_string()
        }

        // define __init__
        #[new]
        fn new(a: String, b: String, other: Py<PyAny>) -> Self {
            Self(Bar::new(a, b, other))
        }

        #[getter]
        fn other(&self) -> Py<PyAny> {
            // Rebind the value of other to a GIL instance, allowing us to clone it
            Python::attach(|py| self.0.other.clone_ref(py))
        }
        // Could also be done as the following
        // #[getter]
        // fn other(&self) -> &Py<PyAny> {
        //     &self.0.other
        // }

        #[setter]
        fn set_other(&mut self, other: Py<PyAny>) {
            self.0.other = other;
        }

        // This is a class instance method. It's wrapper is handled by `pymethods`
        fn combo_length(&self) -> usize {
            self.0.combo_length()
        }

        // Create a class method, binding to the live Python object `cls` that is passed in
        // PyResult allows us to neatly hand exceptions back up to python without having
        // the exceptions causing problems in the FFI.
        #[classmethod]
        fn cls_demo(cls: &Bound<'_, PyType>) -> PyResult<String> {
            let res: PyResult<Bound<'_, PyAny>> = cls.getattr("cls_attr");
            res.map(|s| format!("Successfully called a class method: {}", s))
        }

        // Create a static method
        #[staticmethod]
        fn static_demo() -> String {
            Bar::<Py<PyAny>>::static_demo()
        }

        #[staticmethod]
        fn foo(input: String) -> String {
            let p = crate::parser::foo();
            // let input = "cd['e']fffggg";
            match p.parse(input.as_str()) {
                Ok((s, c)) => format!("found {}, remaining {:?}", c, s),
                Err(e) => format!("error {:?}", e),
            }
        }
        #[staticmethod]
        fn foo_() -> String {
            let p = crate::parser::foo();
            // let input = "cd['e']fffggg";
            match p.parse("+++ab['e']fffgrhfxyxzxyzyzxyxyzxz yzy") {
                Ok((s, c)) => format!("found {}, remaining {:?}", c, s),
                Err(e) => e.to_string(),
            }
        }
    }

    // A value to stand in as null, as we can't use () due to pyo3 restrictions
    #[derive(Clone)]
    #[pyclass]
    pub struct Null {}

    #[derive(Clone)]
    #[pyclass]
    pub enum Json {
        JObject { value: HashMap<String, Json> },
        JArray { value: Vec<Json> },
        JNumber { value: f64 },
        JString { value: String },
        JBool { value: bool },
        JNull { value: Null },
    }

    // Rough and ready JSON parser. This plays fast and loose
    // with whitespace and character parsing. It doesn't even
    // try to handle numbers.

    fn ws<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        satisfy(any_char(), |c| {
            *c == ' ' || *c == '\t' || *c == '\n' || *c == '\r'
        })
        .skip_many()
    }

    fn begin_array<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        ws().seq(char('[')).seq(ws())
    }
    fn end_array<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        ws().seq(char(']')).seq(ws())
    }
    fn begin_object<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        ws().seq(char('{')).seq(ws())
    }
    fn end_object<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        ws().seq(char('}')).seq(ws())
    }
    fn name_separator<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        ws().seq(char(':')).seq(ws())
    }
    fn value_separator<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        ws().seq(char(',')).seq(ws())
    }
    fn quotation_mark<I: Input<Item = char>>() -> impl Parser<I, Output = ()> {
        char('"').void()
    }

    #[pyfunction]
    fn jnull<'s>(input: &'s str) -> ParseResult<&'s str, ()> {
        string("null".to_string()).void().parse(input)
    }
    #[pyfunction]
    fn jbool<'s>(input: &'s str) -> ParseResult<&'s str, bool> {
        string("true".to_string())
            .map(|_| true)
            .or(string("false".to_string()).map(|_| false))
            .parse(input)
    }
    #[pyfunction]
    fn jarray<'s>(input: &'s str) -> ParseResult<&'s str, Vec<Json>> {
        lift(json)
            .sep_by(value_separator())
            .bracket(begin_array(), end_array())
            .parse(input)
    }
    #[pyfunction]
    fn jstring<'s>(input: &'s str) -> ParseResult<&'s str, String> {
        fn is_unescaped(c: &char) -> bool {
            (u32::from(*c) >= 0x20 && u32::from(*c) <= 0x21)
                || (u32::from(*c) >= 0x23 && u32::from(*c) <= 0x5B)
                || (u32::from(*c) >= 0x5D && u32::from(*c) <= 0x10FFFF)
        }
        let unescaped = satisfy(any_char(), is_unescaped);
        let hex_digit = || satisfy(any_char(), |c: &char| c.is_ascii_hexdigit());
        let f = |n: u32| move |c: char| c.to_digit(16).unwrap() * (16 ^ n);

        let escaped = lift(|i| {
            let (i, c) = char('\\').seq(any_char()).parse(i)?;
            let unicode = |i| {
                let (i, u1) = hex_digit().map(f(3)).parse(i)?;
                let (i, u2) = hex_digit().map(f(2)).parse(i)?;
                let (i, u3) = hex_digit().map(f(1)).parse(i)?;
                let (i, u4) = hex_digit().map(f(0)).parse(i)?;
                Result::Ok((i, char::from_u32(u1 + u2 + u3 + u4).unwrap()))
            };
            match c {
                '"' => Result::Ok((i, c)),
                '\\' => Result::Ok((i, c)),
                '/' => Result::Ok((i, c)),
                'b' => Result::Ok((i, char::from(8u8))), // backspace
                'f' => Result::Ok((i, char::from(12u8))), // form feed
                'n' => Result::Ok((i, '\n')),
                'r' => Result::Ok((i, '\r')),
                't' => Result::Ok((i, '\t')),
                'u' => unicode(i),
                _ => unexpected(format!("Unexpected char: '{}'", c)).parse(i),
            }
        });
        unescaped
            .or(escaped)
            .many()
            .surround(quotation_mark())
            .map(|l: Vec<char>| l.iter().collect())
            .parse(input)
    }
    #[pyfunction]
    fn jobject<'s>(input: &'s str) -> ParseResult<&'s str, HashMap<String, Json>> {
        let member = lift_a_2(
            |k, v| (k, v),
            lift(jstring),
            name_separator().seq(lift(json)),
        );

        member
            .sep_by(value_separator())
            .bracket(begin_object(), end_object())
            .map(|l| l.into_iter().collect())
            .parse(input)
    }
    #[pyfunction]
    fn jnumber<'s>(input: &'s str) -> ParseResult<&'s str, f64> {
        let e = char('e').or(char('E'));
        let digit = || any_char().satisfy(|c: &char| c.is_digit(10));
        let digit_1_9 = any_char().satisfy(|c| u32::from(*c) >= 0x31 && u32::from(*c) <= 0x39);
        let exp = |i| {
            let (i, e_) = e.parse(i)?;
            let (i, s) = char('-').or(char('+')).optional().parse(i)?;
            let (i, d) = digit().some().parse(i)?;
            let ds = d.into_iter().collect::<String>();
            Result::Ok((
                i,
                match s {
                    None => format!("{}{}", e_, ds),
                    Some(s_) => format!("{}{}{}", e_, s_, ds),
                },
            ))
        };
        let frac = |i| {
            let (i, p) = char('.').parse(i)?;
            let (i, d) = digit().some().parse(i)?;
            Ok((i, format!("{}{}", p, d.into_iter().collect::<String>())))
        };
        let int = char('0').map(|c| format!("{}", c)).or(digit_1_9.and_then(|c| {
            digit().many().map(move |cs| {
                let mut res = format!("{}", c);
                let mut cs_ : String = cs.into_iter().collect();
                res.push_str(&mut cs_);
                res
            })
        }));
        let number = |i| {
            let (i, m) = char('-').optional().parse(i)?;
            let (i, mut i_) = int.parse(i)?;
            let (i, f) = lift(frac).optional().parse(i)?;
            let (i, e) = lift(exp).optional().parse(i)?;
            let mut m_ = m.map_or_else(|| "".to_string(), |c| format!("{}", c));
            let mut f_ = f.unwrap_or("".to_string());
            let mut e_ = e.unwrap_or("".to_string());
            let mut res = "".to_string();
            res.push_str(&mut m_);
            res.push_str(&mut i_);
            res.push_str(&mut f_);
            res.push_str(&mut e_);
            Ok((i, res))
        };
        let (i, n) = number(input)?;
        Result::Ok((i, n.parse().unwrap()))
    }
    #[pyfunction]
    fn json<'s>(input: &'s str) -> ParseResult<&'s str, Json> {
        lift(jnull)
            .map(|_| Json::JNull { value: Null {} })
            .or(lift(jbool).map(|x| Json::JBool { value: x }))
            .or(lift(jstring).map(|x| Json::JString { value: x }))
            .or(lift(jnumber).map(|x| Json::JNumber { value: x }))
            .or(lift(jarray).map(|x| Json::JArray { value: x }))
            .or(lift(jobject).map(|x| Json::JObject { value: x }))
            .parse(input)
    }
}
