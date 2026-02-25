// Pure rust code, not a python in sight
pub fn sum_as_string(a: usize, b: usize) -> String {
  (a + b).to_string()
}

pub struct Bar<A> {
  pub foo: String,
  pub bar: String,
  pub other: A
}
impl<A> Bar<A> {
  pub fn new(a:String, b:String, other: A) -> Self {
      Self{foo: a, bar: b, other}
  }

  pub fn combo_length(&self) -> usize {
    self.foo.len() + self.bar.len()
  }

  pub fn static_demo() -> String {
    "Successfully called a static method".to_string()
  }
}