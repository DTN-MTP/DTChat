fn main() {
    prost_build::compile_protos(&["src/network/protocols/proto/message.proto"], &["src"]).unwrap();
}
