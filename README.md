Rust structs that can reference their own fields.
===========

Current state: Alpha / experimental.

Background:
-----------

Say that you have a String, some slices of that string and then you want to store both the String and the slices in the same struct:

```rust
pub struct Foo {
    data: String,
    found_items: Vec<&str>,
}
```

You have a few options but all have their drawbacks:
 * store index and length instead of &str - this works for very simple examples such as the above, but does not scale (e g, if there are many different owner Strings, things grow increasingly complex). It also does not work for a lot of other scenarios where the relationship between owner and borrowed pointer is less obvious.
 * go unsafe - this is bad because it is unsafe.
 * [self-borrowing for life](https://www.rust-lang.org/faq.html#how-can-i-define-a-struct-that-contains-a-reference-to-one-of-its-own-fields) - makes the struct permanently borrowed by itself, so you never modify it again.
 * use Rc/Arc together with [owning_ref](https://crates.io/crates/owning_ref) - can cause some overhead, as well as risk for memory leaks via Rc cycles.

The idea:
---------

A complex self-referencing struct is likely to be built once and then used often, but not mutated. It is created in a specific field order, and destroyed in the reverse field order. A field can contain references to fields created earlier, but not later. Also, the struct is always on the heap to ensure it is never moved in memory.

So, we have a crate generate some code that is a safe abstraction around this idea. The code is generated through a build.rs script because of Rust macro [limitations](https://github.com/rust-lang/rust/issues/34303).

Getting started
==============

The build script
---------------

First, you have to set up a [build script](http://doc.crates.io/build-script.html).

Your build.rs would look like this:

```rust
extern crate refstruct;

fn main() {
   refstruct::Scanner::process_src().unwrap();
}
```

And here's what you need to add to Cargo.toml to make the build script execute:

```toml
[package]
build = "build.rs"

[dependencies]
refstruct = "0.1"

[build-dependencies]
refstruct = "0.1"
```

The macro
---------

Second, inside your code, create `refstruct!` macros to create a struct which can reference its own fields. Because my parser is really dumb (for now), please write it like this:

```rust
#[macro_use]
extern crate refstruct;

include!(refstruct!(r#"
// Content of macro, see below
#"));
```

The content of the macro needs to be written in [TOML](https://github.com/toml-lang/toml) format. If you wanted your struct to look like this:

```rust
pub struct Foo {
    data: String,
    found_items: Vec<&str>,
}
```

Here's what the corresponding macro looks like:

```rust
include!(refstruct!(r#"
name = "Foo"
fields = [
    ["data": "String"],
    ["found_items": "Vec<& '_ str>"]
]
#"));
```

Notice the unnamed lifetime `'_` - this indicates that the fields references an earlier field.

Using the generated code
------------------------

```rust
// Start building the struct, this takes the first field (data) as parameter.
let f1 = Foo::new("ABCDEFG".into());

// Calculate the second field from the first.
// Notice how me must use the parameter sent into the closure - we cannot use f1 here. 
let f2 = f1.found_items(|f| vec![&f.data()[0..2], &f.data()[4..6]]);

// Finish building the struct.
let foo: Foo = f2.build();

// foo has accessors for all struct fields.
assert_eq!(foo.data(), String::from("ABCDEFG"));
assert_eq!(foo.found_items(), vec!["AB", "EF"]);
```

