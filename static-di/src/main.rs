use static_di::*;

// How does user specify collection?
// We can add something like #[derive(ResolvedServiceCollection)]
// for structs that are created manually
pub struct MyServiceCollection<T1, T2>
where
    T1: HelloService + 'static,
    T2: GoodbyeService + 'static,
{
    pub s1: T1,
    pub s2: T2,
}

impl<T1, T2> ResolvedServiceCollection for MyServiceCollection<T1, T2>
where
    T1: HelloService + 'static,
    T2: GoodbyeService + 'static,
{
    type S1 = T1;
    type S2 = T2;
    fn t1(&self) -> &T1 {
        &self.s1
    }

    fn t2(&self) -> &T2 {
        &self.s2
    }
}
impl<T: HelloService + 'static, S: GoodbyeService + 'static> HelloServiceProvider
    for MyServiceCollection<T, S>
{
    type HelloService = T;
}
impl<T: HelloService + 'static, S: GoodbyeService + 'static> GoodbyeServiceProvider
    for MyServiceCollection<T, S>
{
    type GoodbyeService = S;
}

pub trait HelloService: Service {
    fn say_hello(&self);
}
// Auto-generated
pub trait HelloServiceProvider {
    type HelloService: HelloService + 'static;
}

pub trait GoodbyeService: Service {
    fn say_goodbye(&self);
}
// Auto-generated
pub trait GoodbyeServiceProvider {
    type GoodbyeService: GoodbyeService + 'static;
}

struct HelloPrinter;
impl Service for HelloPrinter {}
impl HelloService for HelloPrinter {
    fn say_hello(&self) {
        println!("hello");
    }
}

pub struct GoodbyePrinter;
impl Service for GoodbyePrinter {}
impl GoodbyeService for GoodbyePrinter {
    fn say_goodbye(&self) {
        println!("bye-bye.");
    }
}

fn run<C: ServiceCollection + HelloServiceProvider + GoodbyeServiceProvider>(services: C) {
    let hello = services.get_service::<C::HelloService>();
    let goodbye = services.get_service::<C::GoodbyeService>();
    // let hello = collection.
    hello.say_hello();
    goodbye.say_goodbye();
}
fn main() {
    let services = MyServiceCollection {
        s1: HelloPrinter,
        s2: GoodbyePrinter,
    };

    run(services);
}
