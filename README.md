# Setup
## Environment
- Clone the repo
- Install `rust`, preferably via `rustup`
    - `rustup install stable`
- Create and/or active a Python venv
    - `python -m venv .venv`
    - `source .venv/bin/activate`
- Install `maturin`
    - `pipx install maturin`

## Local Development
From the repo root run `maturin develop`. This command will build the rust library, linking it as a python module and installing it into the current venv.

Next run `python` from the venv. From the interpreter you can import the rust module with `import pyo3_test`. Happy testing!

```
>>> import pyo3_test
>>> foo = pyo3_test.Foo
>>> foo.cls_demo()
'Successfully called a class method: cls_attr'
>>> foo.cls_attr = {}
>>> foo.cls_demo()
'Successfully called a class method: {}'
>>> foo.cls_attr = {"qwe":1}
>>> foo.cls_demo()
"Successfully called a class method: {'qwe': 1}"
```

## External Docs

PyO3: https://pyo3.rs/v0.28.2/index.html

Maturin: https://github.com/PyO3/maturin

Rust: https://doc.rust-lang.org/book/