
use std::marker::PhantomData;

pub struct Program(u64);

impl Drop for Program {
    fn drop(&mut self) { println!("Program dropped") }
}

pub struct CommandBuffer<'a>(u64, PhantomData<&'a ()>);

impl<'a> Drop for CommandBuffer<'a> {
    fn drop(&mut self) { println!("CommandBuffer dropped") }
}

impl<'a> CommandBuffer<'a> {
    pub fn new(_program: &'a Program) -> CommandBuffer<'a> { CommandBuffer(23, PhantomData) }
}

include!(refstruct!(r#"
name = "Vulkan"
use = ["super::{Program, CommandBuffer}"]
fields = [
    ["program", "Program"],
    ["command_buffer", "CommandBuffer<'_>"],
]
"#));

pub fn test_vulkan() {
    let v: Vulkan = Vulkan::new(Program(34)).command_buffer(|p| CommandBuffer::new(p.program())).build();

    println!("{} {}", v.program().0, v.command_buffer().0);
}
