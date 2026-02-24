use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod pyo3_test {
    use pyo3::{prelude::*, types::*};

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }

    // Create a basic python class called Foo
    #[pyclass]
    struct Foo {
        foo: String,
        bar: String,
    }
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
        fn new(a:String, b:String) -> Self {
            Self{foo: a, bar: b}
        }

        // This is a class instance method. It's wrapper is handled by `pymethods`
        fn combo_length(&self) -> usize {
            self.foo.len() + self.bar.len()
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
            "Successfully called a static method".to_string()
        }
    }
}
