
use std::marker::PhantomData;

pub struct Device(u64);

impl Drop for Device {
    fn drop(&mut self) { println!("Device {} dropped", self.0) }
}


pub struct Program<'a>(u64, PhantomData<&'a ()>);

impl<'a> Drop for Program<'a> {
    fn drop(&mut self) { println!("Program {} dropped", self.0) }
}

impl<'a> Program<'a> {
    pub fn new(device: &'a Device) -> Program<'a> { Program(device.0 + 10, PhantomData) }
}

pub struct CommandBuffer<'a>(u64, PhantomData<&'a ()>);

impl<'a> Drop for CommandBuffer<'a> {
    fn drop(&mut self) { println!("CommandBuffer {} dropped", self.0) }
}

impl<'a> CommandBuffer<'a> {
    pub fn new(program: &'a Program) -> CommandBuffer<'a> { CommandBuffer(program.0 + 20, PhantomData) }
}

include!(refstruct!(r#"
name = "Vulkan"
namespace = "vk"
module = "vulkanbuilder"
use = ["super::{Device, Program, CommandBuffer}"]
fields = [
    ["device", "Device"],
    ["program", "Program<'_>"],
    ["command_buffer", "CommandBuffer<'_>"],
]
"#));

pub fn test_vulkan() {
    let v: Vulkan = Vulkan::new(Device(12))
        .program(|v| Program::new(v.device()))
        .command_buffer(|v| CommandBuffer::new(v.program()))
        .build();

    println!("Vulkan device {}, program {}, command_buffer {}", v.device().0, v.program().0, v.command_buffer().0);
}
