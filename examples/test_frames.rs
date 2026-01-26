//! Test to show exact S101 frames generated

use ember_plus::s101::S101Encoder;
use ember_plus::codec;
use ember_plus::glow::{GlowRoot, GlowElement, GlowCommand, CommandBuilder};

fn main() {
    // Test 1: Keep-alive request
    let keepalive = S101Encoder::encode_keepalive_request();
    println!("Keep-alive request:");
    print_hex(&keepalive);
    println!("Expected: fe 00 0e 01 01 94 e4 ff\n");

    // Test 2: GetDirectory command (root)
    let elements = vec![GlowElement::Command(GlowCommand::GetDirectory)];
    let root = GlowRoot::with_elements(elements);
    let glow_bytes = codec::encode(&root).unwrap();
    
    println!("GetDirectory BER payload:");
    print_hex(&glow_bytes);
    
    let frame = S101Encoder::encode_ember_packet(&glow_bytes);
    println!("\nGetDirectory S101 frame:");
    print_hex(&frame);
    
    // Test 3: GetDirectory at path [0]
    let elements2 = CommandBuilder::get_directory_at_path(&vec![0]);
    let root2 = GlowRoot::with_elements(elements2);
    let glow_bytes2 = codec::encode(&root2).unwrap();
    
    println!("\nGetDirectory at [0] BER payload:");
    print_hex(&glow_bytes2);
    
    let frame2 = S101Encoder::encode_ember_packet(&glow_bytes2);
    println!("\nGetDirectory at [0] S101 frame:");
    print_hex(&frame2);
    
    // Test 4: GetDirectory at path [5] with identifier "facades"
    let elements3 = CommandBuilder::get_directory_at_path_with_info(&vec![5], Some("facades"));
    let root3 = GlowRoot::with_elements(elements3);
    let glow_bytes3 = codec::encode(&root3).unwrap();
    
    println!("\nGetDirectory at [5] with id='facades' BER payload:");
    print_hex(&glow_bytes3);
    
    let frame3 = S101Encoder::encode_ember_packet(&glow_bytes3);
    println!("\nOur S101 frame:");
    print_hex(&frame3);
    
    println!("\nemberplus-connection frame for [5]:");
    println!("fe000e0001c001021f0260276b25a0236a21a0030d0105a10d310ba0090c07666163616465a20b6409a0076205a003020120...");
    println!("BER breakdown:");
    println!("  60 27 = Root");
    println!("  6b 25 = RootElementCollection");
    println!("  a0 23 = context[0] (NumberedTreeNode)");
    println!("  6a 21 = QualifiedNode APP 10");
    println!("  a0 03 0d 01 05 = path [5]");
    println!("  a1 0d 31 0b a0 09 0c 07 'facades' = contents with identifier");
    println!("  a2 0b ... = children with command");
}

fn print_hex(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        println!();
    }
}
