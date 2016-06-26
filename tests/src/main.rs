#[macro_use]
extern crate refbuilder;

include!(ref_struct!(r#"
name = "FooFinder"
fields = [
   ["data", "String"],
   ["foos", "Vec<& '_ str>"],
]
"#));

mod vulkan;

fn main() {
    let f = FooFinder::new("I've got a foo, you've got a Foo, we all got a FOO!".into());
    let f: FooFinder = f.foos(|f| {
        vec![&f.data()[11..14], &f.data()[29..32], &f.data()[47..50]] }).build();
    println!("{}, {:?}", f.data(), f.foos());

    vulkan::test_vulkan();
}
