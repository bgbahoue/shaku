#![allow(non_snake_case)]

extern crate shaku;
#[macro_use] extern crate shaku_derive;

use std::fmt::Debug;

use shaku::ContainerBuilder;
use shaku::Error as DIError;

trait Foo : Debug + Send {
    fn foo(&self) -> String;
}

#[derive(Component, Debug)]
#[interface(Foo)]
struct FooImpl {
    value: String,
    #[inject]
    bar: Box<Bar>,
}

impl Foo for FooImpl {
    fn foo(&self) -> String {
        format!("FooImpl > foo > value = {} ; bar = {}", self.value, self.bar.bar())
    }
}

trait Bar : Debug + Send {
    fn bar(&self) -> String;
}

#[derive(Component, Debug)]
#[interface(Bar)]
struct BarImpl {
    bar_value: String,
}

impl Bar for BarImpl {
    fn bar(&self) -> String {
        format!("BarImpl > bar > bar_value = {}", self.bar_value)
    }
}

#[test]
fn resolving_component_without_parameters_should_err() {
    let mut builder = ContainerBuilder::new();
    builder
        .register_type::<FooImpl>()
        .as_type::<Foo>();
    let mut container = builder.build().unwrap();
    
    let foo = container
        .resolve::<Foo>();

    assert!(foo.is_err());
    if let Err(DIError::ResolveError(err)) = foo {
        assert_eq!(err, "unable to find parameter with name or type for property value");
    } else {
        panic!("unexpected state > foo should be Err");
    }
}

#[test]
fn resolving_component_dependency_without_parameters_should_err() {
    let mut builder = ContainerBuilder::new();
    builder
        .register_type::<FooImpl>()
        .as_type::<Foo>()
        .with_named_parameter("value", "world is foo".to_string());

    builder
        .register_type::<BarImpl>()
        .as_type::<Bar>();
    let mut container = builder.build().unwrap();
    
    let foo = container
        .resolve::<Foo>();

    assert!(foo.is_err());
    if let Err(DIError::ResolveError(err)) = foo {
        assert_eq!(err, "unable to resolve component for dependency bar");
    } else {
        panic!("unexpected state > foo should be Err");
    }
}

#[test]
fn resolving_component_dependency_with_parameters_dont_err() {
    let mut builder = ContainerBuilder::new();
    builder
        .register_type::<FooImpl>()
        .as_type::<Foo>()
        .with_named_parameter("value", "world is foo".to_string());

    builder
        .register_type::<BarImpl>()
        .as_type::<Bar>();
    let mut container = builder.build().unwrap();
    
    let foo = container
        .with_named_parameter::<Bar, String>("bar_value", "world is bar".to_string())
        .resolve::<Foo>();
    assert_eq!(foo.unwrap().foo(), "FooImpl > foo > value = world is foo ; bar = BarImpl > bar > bar_value = world is bar");
}