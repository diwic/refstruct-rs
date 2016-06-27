#[macro_use]
extern crate refstruct;

include!(refstruct!(r#"
name = "FooFinder"
namespace = "bar"
fields = [
   ["data", "String"],
   ["foos", "Vec<& '_ str>"],
]
"#));

mod vulkan;

include!(refstruct!(r#"
name = "Foo"
fields = [
    ["data", "String"],
    ["found_items", "Vec<& '_ str>"]
]
"#));

fn readme_example() {
    // Start building the struct, this takes the first field (data) as parameter.
    let f1 = Foo::new("ABCDEFG".into());

    // Calculate the second field from the first.
    // Notice how me must use the parameter sent into the closure - we cannot use f1 here. 
    let f2 = f1.found_items(|f| vec![&f.data()[0..2], &f.data()[4..6]]);

    // Finish building the struct.
    let foo: Foo = f2.build();

    // foo has accessors for all struct fields.
    assert_eq!(foo.data(), &String::from("ABCDEFG"));
    assert_eq!(foo.found_items(), &vec!["AB", "EF"]);
}


fn main() {
    let f = FooFinder::new("I've got a foo, you've got a Foo, we all got a FOO!".into());
    let f: FooFinder = f.foos(|f| {
        vec![&f.data()[11..14], &f.data()[29..32], &f.data()[47..50]] }).build();
    println!("{}, {:?}", f.data(), f.foos());
    readme_example();
    vulkan::test_vulkan();
}
