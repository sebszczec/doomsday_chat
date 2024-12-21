
pub trait Interface1 {
    fn foo_bar(self);
}

struct Implementation1;
impl Interface1 for Implementation1 {
    fn foo_bar(self) {
        info!("---> Success1 <---");
    }
}

struct TestStruct1<T> {
    t : T,
}

impl<T : Interface1> TestStruct1<T>
{
    fn new(a : T) -> Self {
        Self { t : a}
    }
    fn test_bar(self) {
        info!("---> Almost Success1 <---");
        self.t.foo_bar();
    }
}

pub trait Interface2 {
    fn foo_bar();
}

struct Implementation2;
impl Interface2 for Implementation2 {
    fn foo_bar() {
        info!("---> Success2 <---");
    }
}

use std::marker::PhantomData;
struct TestStruct2<T> {
    phantom: PhantomData<T>,
}

impl<T : Interface2> TestStruct2<T>
{
    fn test_bar() {
        info!("---> Almost Success2 <---");
        T::foo_bar();
    }
}
