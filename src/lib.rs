mod foo;
mod parser;
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod pyo3_test {
    use pyo3::{prelude::*, types::*};
    use crate::{foo::{self, Bar}, parser::*};

    /// Formats the sum of two numbers as string.
    /// Mixing python macros and pure rust library code
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok(foo::sum_as_string(a, b))
    }

    // Direct python facing code
    #[pyfunction]
    fn fold_list<'py>(list:Bound<'py, PyList>) -> PyResult<i32> {
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
        fn new(a:String, b:String, other: Py<PyAny>) -> Self {
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
            res.map(|s| {
                format!("Successfully called a class method: {}", s)
            })
        }

        // Create a static method
        #[staticmethod]
        fn static_demo() -> String {
            Bar::<Py<PyAny>>::static_demo()
        }

        #[staticmethod]
        fn foo(input:String) -> String {
            let p = crate::parser::foo::<&str>();
            // let input = "cd['e']fffggg";
            match p.parse(&input.as_str()) {
                Ok((s, c)) => format!("found {}, remaining {:?}", c, s),
                Err(e) => format!("error {:?}", e)
            }
        }
                #[staticmethod]
        fn foo_() -> String {
            let p = crate::parser::foo::<&str>();
            // let input = "cd['e']fffggg";
            match p.parse(&"ab['e']fffgrhfxyxzxyzyzxyxyzxzyzy") {
                Ok((s, c)) => format!("found {}, remaining {:?}", c, s),
                Err(e) => format!("error {:?}", e)
            }
        }
    }
}
